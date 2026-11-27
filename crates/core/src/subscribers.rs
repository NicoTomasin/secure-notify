// SPDX-License-Identifier: AGPL-3.0-or-later

//! Subscribers, pairing, and invites (docs/architecture/subscribers.md).
//!
//! The MVP admits person subscribers in private chats: the first admin pairs with the one-time
//! code printed at first boot, and admins admit others with single-use, expiring invite codes.

use std::future::Future;
use std::sync::Arc;

use jiff::{SignedDuration, Timestamp};

use crate::ports::{Clock, InstanceStore, Random, StoreError};

/// How long an invite code stays valid.
pub const INVITE_VALIDITY: SignedDuration = SignedDuration::from_hours(7 * 24);
/// Length of generated codes.
pub const CODE_LENGTH: usize = 10;
/// Key of the pairing code in the instance state.
pub const PAIRING_CODE_KEY: &str = "pairing_code";
/// Crockford base32 alphabet: 32 symbols, so a random byte maps without bias, and no `I`, `L`,
/// `O`, or `U` to confuse.
const CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Internal subscriber ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubscriberId(pub i64);

/// Instance role of a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Can manage access and see instance status.
    Admin,
    /// A regular subscriber.
    Member,
}

impl Role {
    /// The role's stored name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}

/// A subscriber as seen by commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscriber {
    /// Internal ID.
    pub id: SubscriberId,
    /// Instance role.
    pub role: Role,
    /// Bot language.
    pub language: String,
    /// When the subscriber joined.
    pub created_at: Timestamp,
}

/// An interactive endpoint: a chat on a platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointRef {
    /// Platform ID (`telegram`, `console`).
    pub platform: String,
    /// Address on the platform (chat ID).
    pub address: String,
}

/// Summary of a subscriber for `/subscribers`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriberSummary {
    /// The subscriber.
    pub subscriber: Subscriber,
    /// Platforms of its active endpoints.
    pub platforms: Vec<String>,
    /// Whether the subscriber is active.
    pub active: bool,
}

/// Result of redeeming a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Joined {
    /// A new subscriber was created.
    Joined(Subscriber),
    /// The code is unknown, used, or expired; the chat stays unregistered.
    InvalidCode,
}

/// Persistence for subscribers. Operations that check and change state are atomic.
pub trait SubscriberStore: Send + Sync {
    /// The active subscriber owning the active endpoint, if any.
    fn subscriber_by_endpoint(
        &self,
        endpoint: &EndpointRef,
    ) -> impl Future<Output = Result<Option<Subscriber>, StoreError>> + Send;

    /// Whether an admin exists.
    fn admin_exists(&self) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// If `code` equals the stored pairing code and no admin exists, creates the admin with
    /// `endpoint` as primary endpoint and deletes the pairing code, atomically.
    fn pair(
        &self,
        code: &str,
        endpoint: &EndpointRef,
        language: &str,
        now: Timestamp,
    ) -> impl Future<Output = Result<Joined, StoreError>> + Send;

    /// Stores a new invite.
    fn create_invite(
        &self,
        code: &str,
        created_by: SubscriberId,
        now: Timestamp,
        expires_at: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// If `code` is an unused, unexpired invite, creates a member with `endpoint` as primary
    /// endpoint and marks the invite used, atomically.
    fn redeem_invite(
        &self,
        code: &str,
        endpoint: &EndpointRef,
        language: &str,
        now: Timestamp,
    ) -> impl Future<Output = Result<Joined, StoreError>> + Send;

    /// Every subscriber, oldest first.
    fn list_subscribers(
        &self,
    ) -> impl Future<Output = Result<Vec<SubscriberSummary>, StoreError>> + Send;

    /// Deletes a subscriber and everything it owns. Returns whether it existed.
    fn remove_subscriber(
        &self,
        id: SubscriberId,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Sets the bot language of a subscriber.
    fn set_language(
        &self,
        id: SubscriberId,
        language: &str,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// Why removing a subscriber was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveOutcome {
    /// Removed.
    Removed,
    /// No subscriber with that ID.
    NotFound,
    /// Admins cannot remove themselves, so an instance never loses its last admin this way.
    IsSelf,
}

/// Subscriber operations used by the conversation layer.
pub struct Subscribers<S> {
    store: S,
    clock: Arc<dyn Clock>,
    random: Arc<dyn Random>,
}

impl<S: SubscriberStore + InstanceStore> Subscribers<S> {
    /// Creates the service.
    pub fn new(store: S, clock: Arc<dyn Clock>, random: Arc<dyn Random>) -> Self {
        Self {
            store,
            clock,
            random,
        }
    }

    /// The store, for other services built on the same adapter.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Generates a code from the secure random source.
    pub fn generate_code(&self) -> String {
        let mut bytes = [0u8; CODE_LENGTH];
        self.random.fill(&mut bytes);
        bytes
            .iter()
            .map(|b| char::from(CODE_ALPHABET[usize::from(b % 32)]))
            .collect()
    }

    /// Ensures a pairing code exists while the instance has no admin and returns it, so the
    /// binary can print it to its logs. Returns `None` once an admin exists.
    pub async fn ensure_pairing_code(&self) -> Result<Option<String>, StoreError> {
        if self.store.admin_exists().await? {
            return Ok(None);
        }
        if let Some(code) = self.store.instance_value(PAIRING_CODE_KEY).await? {
            return Ok(Some(code));
        }
        let code = self.generate_code();
        self.store
            .set_instance_value(PAIRING_CODE_KEY, &code)
            .await?;
        Ok(Some(code))
    }

    /// The registered subscriber of an endpoint.
    pub async fn resolve(&self, endpoint: &EndpointRef) -> Result<Option<Subscriber>, StoreError> {
        self.store.subscriber_by_endpoint(endpoint).await
    }

    /// `/start <code>` from an unregistered private chat: pairing first, then invites.
    pub async fn join(
        &self,
        code: &str,
        endpoint: &EndpointRef,
        language: &str,
    ) -> Result<Joined, StoreError> {
        let code = normalize_code(code);
        if code.len() != CODE_LENGTH {
            return Ok(Joined::InvalidCode);
        }
        let now = self.clock.now();
        match self.store.pair(&code, endpoint, language, now).await? {
            Joined::InvalidCode => {
                self.store
                    .redeem_invite(&code, endpoint, language, now)
                    .await
            }
            joined @ Joined::Joined(_) => Ok(joined),
        }
    }

    /// `/invite`: a new single-use invite and its expiry.
    pub async fn create_invite(
        &self,
        admin: SubscriberId,
    ) -> Result<(String, Timestamp), StoreError> {
        let now = self.clock.now();
        let expires_at = now
            .checked_add(INVITE_VALIDITY)
            .map_err(|e| StoreError(e.to_string()))?;
        let code = self.generate_code();
        self.store
            .create_invite(&code, admin, now, expires_at)
            .await?;
        Ok((code, expires_at))
    }

    /// `/subscribers`.
    pub async fn list(&self) -> Result<Vec<SubscriberSummary>, StoreError> {
        self.store.list_subscribers().await
    }

    /// `/remove <id>`.
    pub async fn remove(
        &self,
        admin: SubscriberId,
        target: SubscriberId,
    ) -> Result<RemoveOutcome, StoreError> {
        if admin == target {
            return Ok(RemoveOutcome::IsSelf);
        }
        Ok(if self.store.remove_subscriber(target).await? {
            RemoveOutcome::Removed
        } else {
            RemoveOutcome::NotFound
        })
    }

    /// `/language <code>`.
    pub async fn set_language(&self, id: SubscriberId, language: &str) -> Result<(), StoreError> {
        self.store.set_language(id, language).await
    }
}

/// Codes are shown uppercase; typed codes are accepted in any case and with surrounding spaces.
fn normalize_code(code: &str) -> String {
    code.trim().to_ascii_uppercase()
}
