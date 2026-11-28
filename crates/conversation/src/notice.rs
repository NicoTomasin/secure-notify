// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notifications as platform-neutral messages (docs/architecture/delivery.md#message-content,
//! #inline-actions). Source content (titles, summaries, URLs) is passed through as published;
//! platform renderers escape it.

use fluent_bundle::FluentArgs;
use secure_notify_core::app::Store;
use secure_notify_core::delivery::Notice;
use secure_notify_core::matching::{NoticeKind, Relevance};
use secure_notify_core::severity;

use crate::handler::Conversation;
use crate::message::{Action, Block, Message};

/// Longest summary shown in a notification.
const MAX_SUMMARY_CHARS: usize = 300;

impl<S: Store> Conversation<S> {
    fn entity_name(&self, id: &str) -> String {
        self.core()
            .watches
            .taxonomy()
            .get(id)
            .map_or_else(|| id.to_string(), |e| e.name.clone())
    }

    /// Renders a notification for a subscriber who reads `language`.
    pub fn render_notice(&self, notice: &Notice, language: &str) -> Message {
        let i18n = self.i18n();
        let story = &notice.story;
        let text = |key: &str, args: Option<&FluentArgs<'_>>| i18n.text(language, key, args);
        let mut header = FluentArgs::new();
        header.set(
            "severity",
            text(&format!("severity-{}", story.severity.as_str()), None),
        );
        header.set("kind", text(&format!("kind-{}", story.kind.as_str()), None));
        header.set("title", story.title.clone());
        let header_key = match notice.kind {
            NoticeKind::New => "notice-header",
            NoticeKind::Update => "notice-update-header",
        };
        let mut blocks = vec![Block::Heading(text(header_key, Some(&header)))];
        if story.exploited {
            blocks.push(Block::Text(text("notice-exploited", None)));
        } else if story.exploit_available {
            blocks.push(Block::Text(text("notice-exploit-available", None)));
        }
        if let Some(previous) = &notice.previous {
            let mut changes = Vec::new();
            if severity::increased(previous.severity, story.severity) {
                let mut args = FluentArgs::new();
                args.set(
                    "severity",
                    text(&format!("severity-{}", story.severity.as_str()), None),
                );
                changes.push(text("change-severity", Some(&args)));
            }
            if story.exploited && !previous.exploited {
                changes.push(text("change-exploited", None));
            }
            if story.exploit_available && !previous.exploit_available {
                changes.push(text("change-exploit-available", None));
            }
            if story.fix_available && !previous.fix_available {
                changes.push(text("change-fix-available", None));
            }
            for change in changes {
                blocks.push(Block::Item(change));
            }
        }
        match notice.reason.relevance {
            Relevance::ExactVersion | Relevance::Component => {
                for entry in &notice.reason.entries {
                    let mut args = FluentArgs::new();
                    args.set("component", entry.component.clone());
                    let key = match (&entry.version, entry.fixed.is_empty()) {
                        (Some(version), false) => {
                            args.set("version", version.clone());
                            args.set("fixed", entry.fixed.join(", "));
                            "reason-version-fixed"
                        }
                        (Some(version), true) => {
                            args.set("version", version.clone());
                            "reason-version"
                        }
                        (None, _) => "reason-component",
                    };
                    blocks.push(Block::Text(text(key, Some(&args))));
                }
            }
            _ => {
                for entity in &notice.reason.entities {
                    let mut args = FluentArgs::new();
                    args.set("name", self.entity_name(entity));
                    blocks.push(Block::Text(text("reason-watch", Some(&args))));
                }
            }
        }
        if let Some(summary) = &story.summary {
            blocks.push(Block::Text(
                summary.chars().take(MAX_SUMMARY_CHARS).collect(),
            ));
        }
        if let Some((version, score)) = &story.cvss {
            blocks.push(Block::Field {
                label: text("field-cvss", None),
                value: format!("{score:.1} (v{version})"),
            });
        }
        if let Some(epss) = story.epss {
            blocks.push(Block::Field {
                label: text("field-epss", None),
                value: format!("{:.1}%", epss * 100.0),
            });
        }
        let mut sources = FluentArgs::new();
        sources.set("count", story.sources);
        blocks.push(Block::Text(text("notice-sources", Some(&sources))));
        if let Some(url) = &story.url {
            blocks.push(Block::Link {
                label: text("notice-link", None),
                url: url.clone(),
            });
        }
        Message {
            blocks,
            actions: self.notice_actions(notice, language),
            generated_code: None,
        }
    }

    /// Inline actions; each one is a command, which platforms without buttons show as text.
    fn notice_actions(&self, notice: &Notice, language: &str) -> Vec<Action> {
        let i18n = self.i18n();
        let action = |key: &str, args: Option<&FluentArgs<'_>>, command: String| Action {
            label: i18n.text(language, key, args),
            data: command.clone(),
            command,
        };
        let mut actions = vec![action(
            "action-mute-story",
            None,
            format!("/mutes add story {}", notice.story.id.0),
        )];
        for entry in &notice.reason.entries {
            let mut args = FluentArgs::new();
            args.set("component", entry.component.clone());
            actions.push(action(
                "action-mute-component",
                Some(&args),
                format!("/mutes add component {}", entry.component),
            ));
            if let Some(version) = &entry.version {
                args.set("version", version.clone());
                actions.push(action(
                    "action-no-longer-used",
                    Some(&args),
                    format!("/inventory remove {}@{version}", entry.component),
                ));
            }
        }
        for entity in &notice.reason.entities {
            let mut args = FluentArgs::new();
            args.set("name", self.entity_name(entity));
            actions.push(action(
                "action-mute-entity",
                Some(&args),
                format!("/mutes add entity {entity}"),
            ));
        }
        // Buttons only: no command equivalent, so platforms without buttons do not show it.
        actions.push(Action {
            label: i18n.text(language, "action-details", None),
            data: format!("details:{}", notice.story.id.0),
            command: String::new(),
        });
        actions
    }
}
