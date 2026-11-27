// SPDX-License-Identifier: AGPL-3.0-or-later

//! A test instance: the real conversation layer and core services on real SQLite with the real
//! migrations and the repository's content, with a controllable clock.

#![allow(clippy::unwrap_used, dead_code, unreachable_pub)]

use std::path::Path;
use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};
use secure_notify_conversation::{
    About, Attachment, ChatKind, Conversation, I18n, Incoming, render_plain,
};
use secure_notify_core::app::Core;
use secure_notify_core::inventory::Parser;
use secure_notify_core::ports::{Clock, Random};
use secure_notify_core::subscribers::EndpointRef;
use secure_notify_core::taxonomy::Taxonomy;
use secure_notify_parser_bun::BunParser;
use secure_notify_parser_npm::NpmParser;
use secure_notify_parser_pnpm::PnpmParser;
use secure_notify_storage::{Database, OpenOptions};

pub struct TestClock(pub Mutex<Timestamp>);

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

/// Deterministic bytes, so generated codes differ but are reproducible.
pub struct CountingRandom(pub Mutex<u8>);

impl Random for CountingRandom {
    fn fill(&self, buf: &mut [u8]) {
        let mut next = self.0.lock().unwrap();
        for byte in buf {
            *byte = *next;
            *next = next.wrapping_add(7);
        }
    }
}

/// The built-in parsers, as the binary wires them.
pub fn parsers() -> Arc<[Arc<dyn Parser>]> {
    Arc::new([
        Arc::new(NpmParser) as Arc<dyn Parser>,
        Arc::new(PnpmParser),
        Arc::new(BunParser),
    ])
}

/// The taxonomy files of the repository's `content/` directory.
pub fn repository_taxonomy() -> Taxonomy {
    fn walk(dir: &Path, root: &Path, files: &mut Vec<(String, String)>) {
        for path in std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()) {
            if path.is_dir() {
                walk(&path, root, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                files.push((relative, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/taxonomy");
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    Taxonomy::load(&files).unwrap()
}

pub struct Instance {
    pub conversation: Conversation<Database>,
    pub database: Database,
    pub clock: Arc<TestClock>,
    temp_dir: tempfile::TempDir,
}

impl Instance {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let options = OpenOptions {
            path: dir.path().join("db.sqlite3"),
            backup_dir: dir.path().join("backups"),
            now: "2026-11-27T12:00:00Z".parse().unwrap(),
            readers: 2,
        };
        let database = Database::open(&options).unwrap().0;
        let clock = Arc::new(TestClock(Mutex::new(
            "2026-11-27T12:00:00Z".parse().unwrap(),
        )));
        let core = Core::new(
            database.clone(),
            Arc::new(repository_taxonomy()),
            clock.clone(),
            Arc::new(CountingRandom(Mutex::new(1))),
            parsers(),
        );
        let about = About {
            version: "0.0.0-test".to_string(),
            source_url: "https://example.org/source".to_string(),
        };
        Self {
            conversation: Conversation::new(core, I18n::load().unwrap(), about),
            database,
            clock,
            temp_dir: dir,
        }
    }

    /// The data directory of the instance.
    pub fn dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub async fn pairing_code(&self) -> String {
        self.conversation
            .subscribers()
            .ensure_pairing_code()
            .await
            .unwrap()
            .unwrap()
    }

    /// An instance where `alice` is the admin.
    pub async fn with_admin() -> Self {
        let instance = Self::new();
        let code = instance.pairing_code().await;
        instance
            .send("alice", &format!("/start {code}"))
            .await
            .unwrap();
        instance
    }

    pub async fn send(&self, user: &str, text: &str) -> Option<String> {
        self.send_from(user, ChatKind::Private, text, None).await
    }

    pub async fn send_from(
        &self,
        address: &str,
        chat_kind: ChatKind,
        text: &str,
        language_hint: Option<&str>,
    ) -> Option<String> {
        let incoming = Incoming {
            endpoint: EndpointRef {
                platform: "test".to_string(),
                address: address.to_string(),
            },
            chat_kind,
            text: text.to_string(),
            language_hint: language_hint.map(str::to_string),
            attachment: None,
        };
        self.conversation
            .handle(&incoming)
            .await
            .map(|m| render_plain(&m))
    }

    /// Sends a file from a private chat.
    pub async fn upload(&self, user: &str, filename: &str, bytes: Vec<u8>) -> Option<String> {
        let incoming = Incoming {
            endpoint: EndpointRef {
                platform: "test".to_string(),
                address: user.to_string(),
            },
            chat_kind: ChatKind::Private,
            text: String::new(),
            language_hint: None,
            attachment: Some(Attachment {
                filename: filename.to_string(),
                bytes,
            }),
        };
        self.conversation
            .handle(&incoming)
            .await
            .map(|m| render_plain(&m))
    }

    pub async fn invite(&self, admin: &str) -> String {
        let reply = self.send(admin, "/invite").await.unwrap();
        // "Invite code: XXXXXXXXXX" is the first line.
        reply
            .lines()
            .next()
            .unwrap()
            .rsplit(' ')
            .next()
            .unwrap()
            .to_string()
    }

    pub fn advance(&self, by: SignedDuration) {
        let mut now = self.clock.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}
