// SPDX-License-Identifier: AGPL-3.0-or-later

//! Delivery to endpoints: renders notices with the conversation layer and hands them to the
//! platform of each endpoint.

use std::sync::{Arc, Mutex};

use secure_notify_conversation::{Conversation, Message, render_plain};
use secure_notify_core::delivery::{Notice, Outbox};
use secure_notify_core::health::SourceAlert;
use secure_notify_core::ports::SendError;
use secure_notify_core::scan::ScanReport;
use secure_notify_core::sources::BoxFuture;
use secure_notify_core::subscribers::EndpointRef;
use secure_notify_storage::Database;

/// Messages for console endpoints, printed by the console after each step.
#[derive(Default)]
pub(crate) struct ConsoleSink {
    pending: Mutex<Vec<(String, String)>>,
}

impl ConsoleSink {
    /// Takes the messages sent since the last call, as `(address, plain text)`.
    pub(crate) fn drain(&self) -> Vec<(String, String)> {
        std::mem::take(
            &mut *self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }
}

/// The outbox of the instance.
pub(crate) struct PlatformOutbox {
    conversation: Arc<Conversation<Database>>,
    console: Option<Arc<ConsoleSink>>,
}

impl PlatformOutbox {
    /// An outbox over the conversation layer; console endpoints are served when `console` is set.
    pub(crate) fn new(
        conversation: Arc<Conversation<Database>>,
        console: Option<Arc<ConsoleSink>>,
    ) -> Self {
        Self {
            conversation,
            console,
        }
    }
}

impl PlatformOutbox {
    /// Hands a rendered message to the platform of `endpoint`.
    fn send(&self, endpoint: &EndpointRef, message: &Message) -> Result<(), SendError> {
        match (endpoint.platform.as_str(), &self.console) {
            ("console", Some(console)) => {
                console
                    .pending
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((endpoint.address.clone(), render_plain(message)));
                Ok(())
            }
            (platform, _) => Err(SendError::Temporary(format!(
                "no connector for platform {platform}"
            ))),
        }
    }
}

impl Outbox for PlatformOutbox {
    fn deliver<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        notice: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let message = self.conversation.render_notice(notice, language);
            self.send(endpoint, &message)
        })
    }

    fn deliver_report<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        report: &'a ScanReport,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let message = self.conversation.render_scan_report(report, language);
            self.send(endpoint, &message)
        })
    }

    fn deliver_alert<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        alert: &'a SourceAlert,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let message = self.conversation.render_alert(alert, language);
            self.send(endpoint, &message)
        })
    }
}
