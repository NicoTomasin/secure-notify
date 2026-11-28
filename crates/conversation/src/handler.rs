// SPDX-License-Identifier: AGPL-3.0-or-later

//! Command handling: resolves the chat, checks permissions, runs the command through core
//! services, and replies with localized messages.

use fluent_bundle::FluentArgs;
use jiff::Timestamp;
use secure_notify_core::app::{Core, Store};
use secure_notify_core::inventory::{AddOutcome, MAX_COMPONENTS, MAX_UPLOAD_BYTES, UploadOutcome};
use secure_notify_core::matching::{RuleAction, RuleMatch};
use secure_notify_core::ports::StoreError;
use secure_notify_core::subscribers::{
    EndpointRef, Joined, RemoveOutcome, Role, Subscriber, SubscriberId, Subscribers,
};
use secure_notify_core::watches::{UnwatchOutcome, WatchOrigin, WatchOutcome};
use secure_notify_model::SeverityLevel;

use crate::commands::{self, Permission};
use crate::i18n::{I18n, REFERENCE_LANGUAGE};
use crate::message::{Block, CodeKind, GeneratedCode, Message};

/// Attribution required by the NOTICE file; never translated or removed
/// (docs/product/licensing.md#source-offer-and-about).
pub const ATTRIBUTION: &str = concat!(
    "Based on secure-notify (",
    env!("CARGO_PKG_REPOSITORY"),
    ")"
);
/// License name shown by `/about`; never translated.
pub const LICENSE: &str = "AGPL-3.0-or-later";

/// Kind of chat a message comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKind {
    /// A private chat with one person.
    Private,
    /// A group or channel.
    Group,
}

/// A message received by a platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    /// The chat it came from.
    pub endpoint: EndpointRef,
    /// Private or group chat.
    pub chat_kind: ChatKind,
    /// The text.
    pub text: String,
    /// The client language reported by the platform, if any.
    pub language_hint: Option<String>,
    /// A file sent with the message.
    pub attachment: Option<Attachment>,
}

/// A file sent with a message. It is parsed in memory and dropped
/// (docs/architecture/inventory.md#privacy).
#[derive(Clone, PartialEq, Eq)]
pub struct Attachment {
    /// File name as sent.
    pub filename: String,
    /// Content.
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for Attachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print file content.
        write!(f, "Attachment({} bytes)", self.bytes.len())
    }
}

/// What `/about` shows besides the fixed attribution and license.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct About {
    /// Running version.
    pub version: String,
    /// Source code link (`SOURCE_URL`).
    pub source_url: String,
}

/// The conversation layer.
pub struct Conversation<S> {
    core: Core<S>,
    i18n: I18n,
    about: About,
}

impl<S: Store> Conversation<S> {
    /// Creates the conversation layer.
    pub fn new(core: Core<S>, i18n: I18n, about: About) -> Self {
        Self { core, i18n, about }
    }

    /// The core services.
    pub fn core(&self) -> &Core<S> {
        &self.core
    }

    /// The subscriber service.
    pub fn subscribers(&self) -> &Subscribers<S> {
        &self.core.subscribers
    }

    /// What `/about` shows besides the fixed texts.
    pub(crate) fn about_info(&self) -> &About {
        &self.about
    }

    /// The translations.
    pub fn i18n(&self) -> &I18n {
        &self.i18n
    }

    /// Handles a message and returns the reply to the same chat, if any. Messages from chats that
    /// are not registered endpoints get no reply, except `/start` with a valid code
    /// (docs/architecture/subscribers.md#joining-an-instance).
    pub async fn handle(&self, incoming: &Incoming) -> Option<Message> {
        match self.try_handle(incoming).await {
            Ok(reply) => reply,
            Err(error) => {
                tracing::error!(error = %error, "command failed");
                Some(Message::text([self.i18n.text(
                    REFERENCE_LANGUAGE,
                    "error-internal",
                    None,
                )]))
            }
        }
    }

    async fn try_handle(&self, incoming: &Incoming) -> Result<Option<Message>, StoreError> {
        // The MVP serves persons in private chats; group chats are not registered endpoints.
        if incoming.chat_kind != ChatKind::Private {
            return Ok(None);
        }
        let resolved = self.core.subscribers.resolve(&incoming.endpoint).await?;
        if let (Some(subscriber), Some(attachment)) = (&resolved, &incoming.attachment) {
            return Ok(Some(self.upload(subscriber, attachment).await?));
        }
        // Inline action without a command equivalent (platforms with buttons only).
        if let (Some(subscriber), Some(story)) =
            (&resolved, incoming.text.trim().strip_prefix("details:"))
        {
            return Ok(Some(self.details(&subscriber.language, story).await?));
        }
        let Some(line) = commands::parse(&incoming.text) else {
            return Ok(None);
        };
        let Some(subscriber) = resolved else {
            return self.unregistered(incoming, &line).await;
        };
        let language = subscriber.language.clone();
        let Some(spec) = commands::find(&line.name) else {
            return Ok(Some(self.say(&language, "error-unknown-command", None)));
        };
        if spec.permission == Permission::Admin && subscriber.role != Role::Admin {
            let mut args = FluentArgs::new();
            args.set("command", spec.name);
            return Ok(Some(self.say(
                &language,
                "error-permission-denied",
                Some(&args),
            )));
        }
        let reply = match spec.name {
            "start" => self.say(&language, "start-already-registered", None),
            "help" => self.help(&subscriber),
            "about" => self.about(&language),
            "language" => self.language(&subscriber, &line.args).await?,
            "upload" => self.upload_prompt(&language),
            "inventory" => self.inventory(&subscriber, &line.args).await?,
            "rules" => self.rules(&subscriber, &line.args).await?,
            "mutes" => self.mutes(&subscriber, &line.args).await?,
            "status" => self.status(&language).await?,
            "watch" => self.watch(&subscriber, &line.args.join(" ")).await?,
            "unwatch" => self.unwatch(&subscriber, &line.args.join(" ")).await?,
            "watching" => self.watching(&subscriber).await?,
            "invite" => self.invite(&subscriber).await?,
            "subscribers" => self.list(&language).await?,
            "remove" => self.remove(&subscriber, &line.args).await?,
            _ => self.say(&language, "error-unknown-command", None),
        };
        Ok(Some(reply))
    }

    async fn unregistered(
        &self,
        incoming: &Incoming,
        line: &commands::CommandLine<'_>,
    ) -> Result<Option<Message>, StoreError> {
        let (Some(code), "start") = (line.args.first(), line.name.as_str()) else {
            return Ok(None);
        };
        let language = incoming
            .language_hint
            .as_deref()
            .and_then(|hint| self.i18n.match_language(hint))
            .unwrap_or(REFERENCE_LANGUAGE)
            .to_string();
        match self
            .core
            .subscribers
            .join(code, &incoming.endpoint, &language)
            .await?
        {
            Joined::InvalidCode => Ok(None),
            Joined::Joined(subscriber) => {
                let greeting = match subscriber.role {
                    Role::Admin => "start-admin",
                    Role::Member => "start-member",
                };
                Ok(Some(Message::text([
                    self.i18n.text(&language, greeting, None),
                    self.i18n.text(&language, "onboarding", None),
                ])))
            }
        }
    }

    fn say(&self, language: &str, key: &str, args: Option<&FluentArgs<'_>>) -> Message {
        Message::text([self.i18n.text(language, key, args)])
    }

    fn help(&self, subscriber: &Subscriber) -> Message {
        let language = &subscriber.language;
        let mut blocks = vec![Block::Heading(self.i18n.text(
            language,
            "help-header",
            None,
        ))];
        for spec in commands::COMMANDS {
            let allowed = match spec.permission {
                Permission::Anyone => false,
                Permission::Subscriber => true,
                Permission::Admin => subscriber.role == Role::Admin,
            };
            if allowed {
                let mut args = FluentArgs::new();
                args.set("command", spec.name);
                args.set(
                    "description",
                    self.i18n.text(language, spec.description, None),
                );
                blocks.push(Block::Item(self.i18n.text(
                    language,
                    "help-line",
                    Some(&args),
                )));
            }
        }
        Message {
            blocks,
            ..Message::default()
        }
    }

    fn about(&self, language: &str) -> Message {
        let mut version = FluentArgs::new();
        version.set("version", self.about.version.clone());
        let mut license = FluentArgs::new();
        license.set("license", LICENSE);
        let mut source = FluentArgs::new();
        source.set("url", self.about.source_url.clone());
        Message::text([
            ATTRIBUTION.to_string(),
            self.i18n.text(language, "about-license", Some(&license)),
            self.i18n.text(language, "about-version", Some(&version)),
            self.i18n.text(language, "about-source", Some(&source)),
        ])
    }

    fn language_names(&self) -> String {
        self.i18n
            .languages()
            .into_iter()
            .map(|code| format!("{} ({code})", self.i18n.text(code, "language-name", None)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    async fn language(
        &self,
        subscriber: &Subscriber,
        args: &[&str],
    ) -> Result<Message, StoreError> {
        let current = &subscriber.language;
        let available = self.language_names();
        let Some(requested) = args.first() else {
            let mut values = FluentArgs::new();
            values.set("language", self.i18n.text(current, "language-name", None));
            values.set("available", available);
            return Ok(Message::text([
                self.i18n.text(current, "language-current", Some(&values)),
                self.i18n.text(current, "language-usage", None),
            ]));
        };
        let requested = requested.to_ascii_lowercase();
        if !self.i18n.languages().contains(&requested.as_str()) {
            let mut values = FluentArgs::new();
            values.set("code", requested);
            values.set("available", available);
            return Ok(self.say(current, "language-unknown", Some(&values)));
        }
        self.core
            .subscribers
            .set_language(subscriber.id, &requested)
            .await?;
        let mut values = FluentArgs::new();
        values.set(
            "language",
            self.i18n.text(&requested, "language-name", None),
        );
        Ok(self.say(&requested, "language-changed", Some(&values)))
    }

    fn upload_prompt(&self, language: &str) -> Message {
        let mut values = FluentArgs::new();
        values.set("files", self.core.inventory.supported_files().join(", "));
        self.say(language, "upload-prompt", Some(&values))
    }

    async fn upload(
        &self,
        subscriber: &Subscriber,
        attachment: &Attachment,
    ) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let outcome = self
            .core
            .inventory
            .upload(
                subscriber.id,
                &self.core.watches,
                &attachment.filename,
                attachment.bytes.clone(),
            )
            .await?;
        let mut values = FluentArgs::new();
        values.set("file", attachment.filename.clone());
        values.set("files", self.core.inventory.supported_files().join(", "));
        let key = match outcome {
            UploadOutcome::Merged {
                counts,
                technologies,
                deferred,
                ..
            } => {
                values.set("added", counts.added);
                values.set("refreshed", counts.refreshed);
                let mut lines = vec![self.i18n.text(language, "upload-done", Some(&values))];
                if !technologies.is_empty() {
                    let names: Vec<String> = technologies
                        .iter()
                        .map(|id| {
                            self.core
                                .watches
                                .taxonomy()
                                .get(id)
                                .map_or_else(|| id.clone(), |e| e.name.clone())
                        })
                        .collect();
                    let mut tech = FluentArgs::new();
                    tech.set("technologies", names.join(", "));
                    lines.push(self.i18n.text(language, "upload-technologies", Some(&tech)));
                }
                if deferred {
                    lines.push(self.i18n.text(language, "upload-scan-deferred", None));
                }
                return Ok(Message::text(lines));
            }
            UploadOutcome::Unsupported => "upload-unsupported",
            UploadOutcome::TooLarge => {
                values.set("limit", MAX_UPLOAD_BYTES / (1024 * 1024));
                "upload-too-large"
            }
            UploadOutcome::TooManyComponents => {
                values.set("limit", MAX_COMPONENTS);
                "upload-too-many"
            }
            UploadOutcome::TimedOut => "upload-timeout",
            UploadOutcome::Malformed => "upload-malformed",
            UploadOutcome::BunBinaryLockfile => "upload-bun-binary",
        };
        Ok(self.say(language, key, Some(&values)))
    }

    async fn inventory(
        &self,
        subscriber: &Subscriber,
        args: &[&str],
    ) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let inventory = &self.core.inventory;
        let mut values = FluentArgs::new();
        let key = match args {
            [] => return self.inventory_list(subscriber).await,
            ["add", component] | ["add", component, _] => {
                let version = args.get(2).copied();
                values.set("component", (*component).to_string());
                values.set(
                    "version",
                    version.map_or_else(
                        || self.i18n.text(language, "inventory-no-version", None),
                        str::to_string,
                    ),
                );
                match inventory.add(subscriber.id, component, version).await? {
                    AddOutcome::Added => "inventory-added",
                    AddOutcome::InvalidComponent(_) => "inventory-invalid",
                }
            }
            ["remove", spec] => {
                values.set("count", inventory.remove(subscriber.id, spec).await?);
                "inventory-removed"
            }
            ["clear"] => {
                values.set("count", inventory.clear(subscriber.id).await?);
                "inventory-cleared"
            }
            _ => "inventory-usage",
        };
        Ok(self.say(language, key, Some(&values)))
    }

    async fn inventory_list(&self, subscriber: &Subscriber) -> Result<Message, StoreError> {
        /// Entries shown in one reply.
        const SHOWN: usize = 50;
        let language = &subscriber.language;
        let entries = self.core.inventory.list(subscriber.id).await?;
        if entries.is_empty() {
            return Ok(self.say(language, "inventory-empty", None));
        }
        let mut count = FluentArgs::new();
        count.set("count", entries.len());
        let mut blocks = vec![Block::Heading(self.i18n.text(
            language,
            "inventory-header",
            Some(&count),
        ))];
        for entry in entries.iter().take(SHOWN) {
            let version = entry
                .version
                .clone()
                .unwrap_or_else(|| self.i18n.text(language, "inventory-no-version", None));
            blocks.push(Block::Item(format!("{} {version}", entry.component)));
        }
        if entries.len() > SHOWN {
            let mut more = FluentArgs::new();
            more.set("count", entries.len() - SHOWN);
            blocks.push(Block::Text(self.i18n.text(
                language,
                "inventory-more",
                Some(&more),
            )));
        }
        blocks.push(Block::Text(self.i18n.text(
            language,
            "inventory-usage",
            None,
        )));
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    /// The "Details" inline action (`details:<story>`): sources, identifiers, affected
    /// components, and related stories that are still retained.
    async fn details(&self, language: &str, story: &str) -> Result<Message, StoreError> {
        let Ok(id) = story.parse::<i64>() else {
            return Ok(self.say(language, "details-gone", None));
        };
        let database = self.core().subscribers.store();
        let Some(details) = database
            .story_details(secure_notify_core::correlation::StoryId(id))
            .await?
        else {
            return Ok(self.say(language, "details-gone", None));
        };
        let mut blocks = vec![Block::Heading(details.title)];
        let mut values = FluentArgs::new();
        values.set("ids", details.identifiers.join(", "));
        blocks.push(Block::Text(self.i18n.text(
            language,
            "details-identifiers",
            Some(&values),
        )));
        let mut values = FluentArgs::new();
        values.set("sources", details.sources.join(", "));
        blocks.push(Block::Text(self.i18n.text(
            language,
            "details-sources",
            Some(&values),
        )));
        for (component, fixed) in details.affected {
            let mut values = FluentArgs::new();
            values.set("component", component);
            let key = if fixed.is_empty() {
                "details-affected-unfixed"
            } else {
                values.set("fixed", fixed.join(", "));
                "details-affected"
            };
            blocks.push(Block::Item(self.i18n.text(language, key, Some(&values))));
        }
        for title in details.related {
            let mut values = FluentArgs::new();
            values.set("title", title);
            blocks.push(Block::Item(self.i18n.text(
                language,
                "details-related",
                Some(&values),
            )));
        }
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    async fn mutes(&self, subscriber: &Subscriber, args: &[&str]) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let service = &self.core().mutes;
        let mut values = FluentArgs::new();
        match args {
            [] => {
                let mutes = service.list(subscriber.id).await?;
                if mutes.is_empty() {
                    return Ok(self.say(language, "mutes-none", None));
                }
                let mut blocks = vec![Block::Heading(self.i18n.text(
                    language,
                    "mutes-header",
                    None,
                ))];
                for mute in mutes {
                    let mut line = FluentArgs::new();
                    line.set("id", mute.id);
                    line.set("scope", mute.scope);
                    line.set("value", mute.value);
                    let key = match mute.until {
                        Some(until) => {
                            line.set("until", format_time(until));
                            "mutes-line-until"
                        }
                        None => "mutes-line",
                    };
                    blocks.push(Block::Item(self.i18n.text(language, key, Some(&line))));
                }
                blocks.push(Block::Text(self.i18n.text(language, "mutes-usage", None)));
                Ok(Message {
                    blocks,
                    ..Message::default()
                })
            }
            ["add", scope, value] | ["add", scope, value, _] => {
                let duration = args.get(3).copied();
                values.set("scope", (*scope).to_string());
                values.set("value", (*value).to_string());
                match service.add(subscriber.id, scope, value, duration).await? {
                    Ok(None) => Ok(self.say(language, "mutes-added", Some(&values))),
                    Ok(Some(until)) => {
                        values.set("until", format_time(until));
                        Ok(self.say(language, "mutes-added-until", Some(&values)))
                    }
                    Err(_) => Ok(Message::text([
                        self.i18n.text(language, "mutes-invalid", None),
                        self.i18n.text(language, "mutes-usage", None),
                    ])),
                }
            }
            ["remove", id] => {
                let Ok(id) = id.trim_start_matches('#').parse::<i64>() else {
                    return Ok(self.say(language, "mutes-usage", None));
                };
                values.set("id", id);
                let key = if service.remove(subscriber.id, id).await? {
                    "mutes-removed"
                } else {
                    "mutes-not-found"
                };
                Ok(self.say(language, key, Some(&values)))
            }
            _ => Ok(self.say(language, "mutes-usage", None)),
        }
    }

    async fn rules(&self, subscriber: &Subscriber, args: &[&str]) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let service = &self.core.rules;
        match args {
            [] => {}
            ["reset"] => {
                service.reset(subscriber.id).await?;
                return Ok(self.say(language, "rules-reset", None));
            }
            ["severity", "off"] => {
                service.set_min_severity(subscriber.id, None).await?;
                return Ok(self.say(language, "rules-severity-off", None));
            }
            ["severity", level] => {
                let mut values = FluentArgs::new();
                values.set("level", level.to_ascii_lowercase());
                let parsed = SeverityLevel::parse(&level.to_ascii_lowercase())
                    .filter(|l| *l != SeverityLevel::Unknown);
                let Some(parsed) = parsed else {
                    return Ok(self.say(language, "rules-severity-invalid", Some(&values)));
                };
                service
                    .set_min_severity(subscriber.id, Some(parsed))
                    .await?;
                return Ok(self.say(language, "rules-severity-set", Some(&values)));
            }
            _ => return Ok(self.say(language, "rules-usage", None)),
        }
        let (rules, defaults) = service.effective(subscriber.id).await?;
        let mut header = FluentArgs::new();
        header.set("default", if defaults { "yes" } else { "no" });
        let mut blocks = vec![Block::Heading(self.i18n.text(
            language,
            "rules-header",
            Some(&header),
        ))];
        for (index, rule) in rules.iter().enumerate() {
            let mut values = FluentArgs::new();
            values.set("position", index + 1);
            values.set("conditions", rule_conditions(&rule.conditions));
            let key = match (rule.action, rule.min_severity) {
                (RuleAction::Ignore, _) => "rule-ignore",
                (RuleAction::Notify, None) => "rule-notify",
                (RuleAction::Notify, Some(level)) => {
                    values.set("level", level.as_str());
                    "rule-notify-min"
                }
            };
            blocks.push(Block::Item(self.i18n.text(language, key, Some(&values))));
        }
        let min = service.min_severity(subscriber.id).await?;
        let mut values = FluentArgs::new();
        let key = match min {
            Some(level) => {
                values.set("level", level.as_str());
                "rules-min-severity"
            }
            None => "rules-min-severity-none",
        };
        blocks.push(Block::Text(self.i18n.text(language, key, Some(&values))));
        blocks.push(Block::Text(self.i18n.text(language, "rules-usage", None)));
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    fn catalog(&self) -> String {
        self.core
            .watches
            .taxonomy()
            .entities()
            .map(|e| e.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }

    async fn watch(&self, subscriber: &Subscriber, query: &str) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let mut values = FluentArgs::new();
        values.set("available", self.catalog());
        if query.is_empty() {
            return Ok(self.say(language, "watch-usage", Some(&values)));
        }
        let key = match self.core.watches.watch(subscriber.id, query).await? {
            WatchOutcome::Added(entity) => {
                values.set("name", entity.name);
                "watch-added"
            }
            WatchOutcome::AlreadyWatched(entity) => {
                values.set("name", entity.name);
                "watch-already"
            }
            WatchOutcome::Unknown => {
                values.set("query", query.to_string());
                "watch-unknown"
            }
        };
        Ok(self.say(language, key, Some(&values)))
    }

    async fn unwatch(&self, subscriber: &Subscriber, query: &str) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let mut values = FluentArgs::new();
        values.set("available", self.catalog());
        if query.is_empty() {
            return Ok(self.say(language, "unwatch-usage", None));
        }
        let key = match self.core.watches.unwatch(subscriber.id, query).await? {
            UnwatchOutcome::Removed(entity) => {
                values.set("name", entity.name);
                "unwatch-removed"
            }
            UnwatchOutcome::NotWatched(entity) => {
                values.set("name", entity.name);
                "unwatch-not-watched"
            }
            UnwatchOutcome::Unknown => {
                values.set("query", query.to_string());
                "watch-unknown"
            }
        };
        Ok(self.say(language, key, Some(&values)))
    }

    async fn watching(&self, subscriber: &Subscriber) -> Result<Message, StoreError> {
        let language = &subscriber.language;
        let watches = self.core.watches.list(subscriber.id).await?;
        if watches.is_empty() {
            return Ok(self.say(language, "watching-none", None));
        }
        let mut blocks = vec![Block::Heading(self.i18n.text(
            language,
            "watching-header",
            None,
        ))];
        for (id, name, origin) in watches {
            let mut values = FluentArgs::new();
            values.set("id", id);
            values.set("name", name);
            values.set(
                "origin",
                match origin {
                    WatchOrigin::Manual => "manual",
                    WatchOrigin::Derived => "derived",
                },
            );
            blocks.push(Block::Item(self.i18n.text(
                language,
                "watching-line",
                Some(&values),
            )));
        }
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    async fn invite(&self, admin: &Subscriber) -> Result<Message, StoreError> {
        let (code, expires) = self.core.subscribers.create_invite(admin.id).await?;
        let language = &admin.language;
        let mut values = FluentArgs::new();
        values.set("code", code.clone());
        values.set("expires", format_time(expires));
        let mut message = Message::text([
            self.i18n.text(language, "invite-created", Some(&values)),
            self.i18n
                .text(language, "invite-instructions", Some(&values)),
        ]);
        message.generated_code = Some(GeneratedCode {
            kind: CodeKind::Invite,
            value: code,
        });
        Ok(message)
    }

    async fn list(&self, language: &str) -> Result<Message, StoreError> {
        let all = self.core.subscribers.list().await?;
        let mut count = FluentArgs::new();
        count.set("count", all.len());
        let mut blocks = vec![Block::Heading(self.i18n.text(
            language,
            "subscribers-header",
            Some(&count),
        ))];
        for summary in all {
            let s = &summary.subscriber;
            let role_key = match s.role {
                Role::Admin => "role-admin",
                Role::Member => "role-member",
            };
            let mut values = FluentArgs::new();
            values.set("id", s.id.0);
            values.set("role", self.i18n.text(language, role_key, None));
            values.set("language", s.language.clone());
            values.set("since", s.created_at.strftime("%Y-%m-%d").to_string());
            values.set("active", if summary.active { "yes" } else { "no" });
            blocks.push(Block::Item(self.i18n.text(
                language,
                "subscribers-line",
                Some(&values),
            )));
        }
        Ok(Message {
            blocks,
            ..Message::default()
        })
    }

    async fn remove(&self, admin: &Subscriber, args: &[&str]) -> Result<Message, StoreError> {
        let language = &admin.language;
        let Some(id) = args
            .first()
            .map(|a| a.trim_start_matches('#'))
            .and_then(|a| a.parse::<i64>().ok())
        else {
            return Ok(self.say(language, "remove-usage", None));
        };
        let mut values = FluentArgs::new();
        values.set("id", id);
        let key = match self
            .core
            .subscribers
            .remove(admin.id, SubscriberId(id))
            .await?
        {
            RemoveOutcome::Removed => "remove-done",
            RemoveOutcome::NotFound => "remove-not-found",
            RemoveOutcome::IsSelf => "remove-self",
        };
        Ok(self.say(language, key, Some(&values)))
    }
}

/// A rule's conditions as technical tokens, the same in every language.
fn rule_conditions(conditions: &RuleMatch) -> String {
    let mut parts = Vec::new();
    if let Some(exploited) = conditions.exploited {
        parts.push(format!("exploited = {exploited}"));
    }
    if let Some(available) = conditions.exploit_available {
        parts.push(format!("exploit_available = {available}"));
    }
    if !conditions.relevance.is_empty() {
        let levels: Vec<&str> = conditions.relevance.iter().map(|r| r.as_str()).collect();
        parts.push(format!("relevance = {}", levels.join(", ")));
    }
    if !conditions.kind.is_empty() {
        let kinds: Vec<&str> = conditions.kind.iter().map(|k| k.as_str()).collect();
        parts.push(format!("kind = {}", kinds.join(", ")));
    }
    if let Some(entity) = &conditions.entity {
        parts.push(format!("entity = {entity}"));
    }
    if let Some(component) = &conditions.component {
        parts.push(format!("component = {component}"));
    }
    if !conditions.source.is_empty() {
        parts.push(format!("source = {}", conditions.source.join(", ")));
    }
    if let Some(min) = conditions.min_epss {
        parts.push(format!("min_epss = {min}"));
    }
    parts.join("; ")
}

/// Times shown to subscribers; the MVP shows UTC.
pub(crate) fn format_time(timestamp: Timestamp) -> String {
    timestamp.strftime("%Y-%m-%d %H:%M UTC").to_string()
}
