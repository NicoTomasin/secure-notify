// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pairing, invites, and admin commands through the conversation layer, on real SQLite with the
//! real migrations (docs/architecture/subscribers.md).

#![allow(clippy::unwrap_used)]

use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};
use secure_notify_conversation::{
    About, ChatKind, Conversation, I18n, Incoming, Message, render_plain,
};
use secure_notify_core::ports::{Clock, InstanceStore, Random};
use secure_notify_core::subscribers::{EndpointRef, Subscribers};
use secure_notify_storage::{Database, OpenOptions};

struct TestClock(Mutex<Timestamp>);

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        *self.0.lock().unwrap()
    }
}

/// Deterministic bytes, so generated codes differ but are reproducible.
struct CountingRandom(Mutex<u8>);

impl Random for CountingRandom {
    fn fill(&self, buf: &mut [u8]) {
        let mut next = self.0.lock().unwrap();
        for byte in buf {
            *byte = *next;
            *next = next.wrapping_add(7);
        }
    }
}

struct Instance {
    conversation: Conversation<Database>,
    clock: Arc<TestClock>,
    _dir: tempfile::TempDir,
}

impl Instance {
    fn new() -> Self {
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
        let subscribers = Subscribers::new(
            database,
            clock.clone(),
            Arc::new(CountingRandom(Mutex::new(1))),
        );
        let about = About {
            version: "0.0.0-test".to_string(),
            source_url: "https://example.org/source".to_string(),
        };
        Self {
            conversation: Conversation::new(subscribers, I18n::load().unwrap(), about),
            clock,
            _dir: dir,
        }
    }

    async fn pairing_code(&self) -> String {
        self.conversation
            .subscribers()
            .ensure_pairing_code()
            .await
            .unwrap()
            .unwrap()
    }

    async fn send(&self, user: &str, text: &str) -> Option<String> {
        self.send_from(user, ChatKind::Private, text, None).await
    }

    async fn send_from(
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
        };
        self.conversation
            .handle(&incoming)
            .await
            .map(|m: Message| render_plain(&m))
    }

    async fn invite(&self, admin: &str) -> String {
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

    fn advance(&self, by: SignedDuration) {
        let mut now = self.clock.0.lock().unwrap();
        *now = now.checked_add(by).unwrap();
    }
}

#[tokio::test]
async fn unregistered_chats_get_no_reply_except_start_with_a_valid_code() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    for text in ["/help", "/about", "/start", "/start WRONGCODE1", "hello"] {
        assert_eq!(instance.send("stranger", text).await, None, "{text}");
    }
    let reply = instance.send("alice", &format!("/start {code}")).await;
    assert!(reply.unwrap().contains("You are now the admin"));
}

#[tokio::test]
async fn the_pairing_code_works_once() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    // Codes are accepted in any case.
    let lowercase = code.to_ascii_lowercase();
    instance
        .send("alice", &format!("/start {lowercase}"))
        .await
        .unwrap();
    // The one-time code is deleted once used.
    let store = instance.conversation.subscribers().store();
    assert_eq!(store.instance_value("pairing_code").await.unwrap(), None);
    // Even a pairing code left behind cannot create a second admin.
    store
        .set_instance_value("pairing_code", "ABCDEFGHJK")
        .await
        .unwrap();
    assert_eq!(instance.send("mallory", "/start ABCDEFGHJK").await, None);
    assert_eq!(
        instance.send("mallory", &format!("/start {code}")).await,
        None
    );
    // Once an admin exists, no new pairing code is created.
    let again = instance
        .conversation
        .subscribers()
        .ensure_pairing_code()
        .await
        .unwrap();
    assert_eq!(again, None);
}

#[tokio::test]
async fn invites_are_single_use_and_expire() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    instance
        .send("alice", &format!("/start {code}"))
        .await
        .unwrap();

    let first = instance.invite("alice").await;
    let joined = instance.send("bob", &format!("/start {first}")).await;
    assert!(joined.unwrap().contains("Welcome"));
    assert_eq!(
        instance.send("carol", &format!("/start {first}")).await,
        None
    );

    let second = instance.invite("alice").await;
    // subscribers.md: invites expire; the MVP validity is seven days.
    instance.advance(SignedDuration::from_hours(7 * 24 + 1));
    assert_eq!(
        instance.send("carol", &format!("/start {second}")).await,
        None
    );
}

#[tokio::test]
async fn members_cannot_run_admin_commands() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    instance
        .send("alice", &format!("/start {code}"))
        .await
        .unwrap();
    let invite = instance.invite("alice").await;
    instance
        .send("bob", &format!("/start {invite}"))
        .await
        .unwrap();
    for command in ["invite", "subscribers", "remove"] {
        let reply = instance
            .send("bob", &format!("/{command} 1"))
            .await
            .unwrap();
        assert_eq!(reply, format!("Only admins can use /{command}."));
    }
    let help = instance.send("bob", "/help").await.unwrap();
    assert!(!help.contains("/invite"), "{help}");
}

#[tokio::test]
async fn group_chats_are_ignored() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    assert_eq!(
        instance
            .send_from(
                "group:devs",
                ChatKind::Group,
                &format!("/start {code}"),
                None
            )
            .await,
        None
    );
}

#[tokio::test]
async fn new_persons_get_the_client_language_when_a_locale_exists() {
    // delivery.md#localization-i18n: default for persons is the platform's reported language.
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    let reply = instance
        .send_from(
            "alice",
            ChatKind::Private,
            &format!("/start {code}"),
            Some("es-AR"),
        )
        .await
        .unwrap();
    assert!(reply.starts_with("Ahora sos admin"), "{reply}");
    let invite = instance.invite("alice").await;
    let reply = instance
        .send_from(
            "bob",
            ChatKind::Private,
            &format!("/start {invite}"),
            Some("fr"),
        )
        .await
        .unwrap();
    assert!(reply.starts_with("Welcome"), "{reply}");
}

#[tokio::test]
async fn about_keeps_attribution_and_license_untranslated() {
    // licensing.md#source-offer-and-about.
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    instance
        .send("alice", &format!("/start {code}"))
        .await
        .unwrap();
    instance.send("alice", "/language es").await.unwrap();
    let about = instance.send("alice", "/about").await.unwrap();
    assert!(
        about.contains("Based on secure-notify (https://github.com/NicoTomasin/secure-notify)")
    );
    assert!(about.contains("AGPL-3.0-or-later"));
    assert!(
        about.contains("https://example.org/source"),
        "SOURCE_URL is the source link"
    );
}

#[tokio::test]
async fn admins_cannot_remove_themselves_but_can_remove_others() {
    let instance = Instance::new();
    let code = instance.pairing_code().await;
    instance
        .send("alice", &format!("/start {code}"))
        .await
        .unwrap();
    let invite = instance.invite("alice").await;
    instance
        .send("bob", &format!("/start {invite}"))
        .await
        .unwrap();
    assert_eq!(
        instance.send("alice", "/remove 1").await.unwrap(),
        "You cannot remove yourself."
    );
    assert_eq!(
        instance.send("alice", "/remove 2").await.unwrap(),
        "Subscriber #2 and their data were removed."
    );
    // The removed person's chat is unregistered again.
    assert_eq!(instance.send("bob", "/help").await, None);
}
