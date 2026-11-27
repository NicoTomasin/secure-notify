// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ports: what the core needs from the outside world. Adapter crates implement them and only the
//! binary wires concrete implementations together
//! (docs/architecture/overview.md#crate-dependencies). Nothing here exposes SQL, HTTP-library,
//! or platform types.

use std::future::Future;

use jiff::Timestamp;

/// Source of the current time, so domain logic is deterministic in tests and the console can
/// advance a development clock.
pub trait Clock: Send + Sync {
    /// The current time, in UTC.
    fn now(&self) -> Timestamp;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// A storage operation failed. The message is for logs and never contains row contents.
#[derive(Debug, thiserror::Error)]
#[error("storage failure: {0}")]
pub struct StoreError(pub String);

/// Instance-wide key/value state, such as the pairing code.
pub trait InstanceStore: Send + Sync {
    /// The value stored under `key`, if any.
    fn instance_value(
        &self,
        key: &str,
    ) -> impl Future<Output = Result<Option<String>, StoreError>> + Send;

    /// Stores `value` under `key`, replacing any previous value.
    fn set_instance_value(
        &self,
        key: &str,
        value: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// An outbound HTTP `GET` request made through the shared client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// Absolute `http` or `https` URL.
    pub url: String,
    /// Extra request headers, such as conditional request validators.
    pub headers: Vec<(String, String)>,
}

/// A response from the shared HTTP client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers, names in lowercase.
    pub headers: Vec<(String, String)>,
    /// Response body, within the client's size limit.
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// The first value of header `name` (lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Why an HTTP request did not produce a response.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HttpError {
    /// The destination is not allowed (undeclared host, private address, bad scheme).
    #[error("request refused: {0}")]
    Refused(String),
    /// The response exceeded the size limit.
    #[error("response larger than {0} bytes")]
    TooLarge(usize),
    /// The request timed out.
    #[error("request timed out")]
    Timeout,
    /// Any other transport failure.
    #[error("transport error: {0}")]
    Transport(String),
}

/// The shared, rate-limited HTTP client (docs/architecture/plugins.md#source-context).
pub trait HttpClient: Send + Sync {
    /// Performs a `GET` request.
    fn get(
        &self,
        request: HttpRequest,
    ) -> impl Future<Output = Result<HttpResponse, HttpError>> + Send;
}

/// Turns something the core produces (a notification, a command reply) into a platform-neutral
/// message, localized for `language`. Implemented by the conversation layer.
pub trait Renderer<T>: Send + Sync {
    /// The platform-neutral message type.
    type Message;

    /// Renders `item` for a subscriber who reads `language`.
    fn render(&self, item: &T, language: &str) -> Self::Message;
}

/// Why a platform could not deliver a message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    /// The bot lost access to the endpoint (blocked, removed from the group); the endpoint is
    /// deactivated (docs/architecture/subscribers.md#subscribers-and-endpoints).
    #[error("endpoint is no longer reachable")]
    EndpointGone,
    /// A failure worth retrying with backoff.
    #[error("temporary failure: {0}")]
    Temporary(String),
}

/// Delivery side of a platform plugin (docs/architecture/platforms.md#platform-plugins).
pub trait Platform: Send + Sync {
    /// The message type the platform renders, produced by a [`Renderer`].
    type Message: Send + Sync;

    /// The platform's identifier, as stored in endpoints (`telegram`, `console`).
    fn id(&self) -> &'static str;

    /// Sends `message` to the endpoint `address` (a chat ID or similar).
    fn send(
        &self,
        address: &str,
        message: &Self::Message,
    ) -> impl Future<Output = Result<(), SendError>> + Send;
}
