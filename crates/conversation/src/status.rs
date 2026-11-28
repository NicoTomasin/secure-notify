// SPDX-License-Identifier: AGPL-3.0-or-later

//! `/status` and failure-streak alerts (docs/operations/self-hosting.md#operations).

use fluent_bundle::FluentArgs;
use secure_notify_core::app::Store;
use secure_notify_core::health::{SourceAlert, SourceHealth};
use secure_notify_core::ports::StoreError;

use crate::handler::{Conversation, format_time};
use crate::message::{Block, Message};

impl<S: Store> Conversation<S> {
    /// `/status` (admins): instance counters and each source's health.
    pub(crate) async fn status(&self, language: &str) -> Result<Message, StoreError> {
        let i18n = self.i18n();
        let text = |key: &str, args: Option<&FluentArgs<'_>>| i18n.text(language, key, args);
        let (sources, stats) = self.core().status.report().await?;
        let mut version = FluentArgs::new();
        version.set("version", self.about_info().version.clone());
        let mut counters = FluentArgs::new();
        counters.set("subscribers", stats.subscribers);
        counters.set("stories", stats.stories);
        counters.set("events", stats.events);
        counters.set("queued", stats.queued);
        counters.set("scans", stats.pending_scans);
        let mut blocks = vec![
            Block::Heading(text("status-header", None)),
            Block::Text(text("status-version", Some(&version))),
            Block::Text(text("status-stats", Some(&counters))),
            Block::Text(text("status-sources", None)),
        ];
        let never = text("status-never", None);
        for source in sources {
            let mut args = FluentArgs::new();
            args.set("name", source.name);
            args.set("items", source.items);
            let key = match source.health {
                SourceHealth::Disabled => "status-source-disabled",
                SourceHealth::FirstSync => "status-source-first-sync",
                SourceHealth::Healthy { last_success } => {
                    args.set("success", format_time(last_success));
                    "status-source-healthy"
                }
                SourceHealth::Failing {
                    failures,
                    last_failure,
                    error,
                    last_success,
                } => {
                    args.set("failures", failures);
                    args.set(
                        "failed",
                        last_failure.map_or_else(|| never.clone(), format_time),
                    );
                    args.set("error", error);
                    args.set(
                        "success",
                        last_success.map_or_else(|| never.clone(), format_time),
                    );
                    "status-source-failing"
                }
            };
            blocks.push(Block::Item(text(key, Some(&args))));
        }
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    /// The alert sent to admins when a source keeps failing.
    pub fn render_alert(&self, alert: &SourceAlert, language: &str) -> Message {
        let mut args = FluentArgs::new();
        args.set("name", alert.source.clone());
        args.set("failures", alert.failures);
        args.set("error", alert.error.clone());
        Message::text([self
            .i18n()
            .text(language, "alert-source-failing", Some(&args))])
    }
}
