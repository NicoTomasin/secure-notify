// SPDX-License-Identifier: AGPL-3.0-or-later

//! Notification rendering per locale (docs/engineering/testing.md#snapshots) and `/mutes`.

#![allow(clippy::unwrap_used)]

mod common;

use common::Instance;
use secure_notify_conversation::render_plain;
use secure_notify_core::correlation::StoryId;
use secure_notify_core::delivery::{Notice, StoryView};
use secure_notify_core::matching::{MatchedEntry, NoticeKind, Reason, Relevance, Snapshot};
use secure_notify_model::{EventKind, SeverityLevel};

/// The real GHSA-35jh-r3h4-6jhm story, as an update notice after it became exploited.
fn notice() -> Notice {
    Notice {
        kind: NoticeKind::Update,
        story: StoryView {
            id: StoryId(1),
            kind: EventKind::Vulnerability,
            title: "Command Injection in lodash".to_string(),
            url: Some("https://osv.dev/vulnerability/GHSA-35jh-r3h4-6jhm".to_string()),
            severity: SeverityLevel::High,
            exploited: true,
            exploit_available: false,
            fix_available: true,
            cvss: Some(("3.1".to_string(), 7.2)),
            epss: None,
            summary: None,
            entities: vec!["tech:node".to_string()],
            sources: 2,
        },
        reason: Reason {
            relevance: Relevance::ExactVersion,
            entries: vec![MatchedEntry {
                component: "pkg:npm/lodash".to_string(),
                version: Some("4.17.20".to_string()),
                fixed: vec!["4.17.21".to_string()],
            }],
            entities: Vec::new(),
        },
        previous: Some(Snapshot {
            severity: SeverityLevel::High,
            exploited: false,
            exploit_available: false,
            fix_available: true,
        }),
    }
}

#[tokio::test]
async fn update_notices_render_in_english_and_spanish() {
    let instance = Instance::new();
    for language in ["en", "es"] {
        let text = render_plain(&instance.conversation.render_notice(&notice(), language));
        insta::assert_snapshot!(format!("update_notice_{language}"), text);
    }
}

#[tokio::test]
async fn mutes_are_added_listed_and_removed() {
    let instance = Instance::with_admin().await;
    assert_eq!(
        instance
            .send("alice", "/mutes add story 12 7d")
            .await
            .unwrap(),
        "Muted story 12 until 2026-12-04 12:00 UTC."
    );
    assert!(
        instance
            .send("alice", "/mutes add story twelve")
            .await
            .unwrap()
            .starts_with("That mute is not valid.")
    );
    assert!(
        instance
            .send("alice", "/mutes add entity tech:node 2x")
            .await
            .unwrap()
            .starts_with("That mute is not valid.")
    );
    instance
        .send("alice", "/mutes add version pkg:npm/lodash@4.17.20")
        .await
        .unwrap();
    let list = instance.send("alice", "/mutes").await.unwrap();
    assert!(
        list.contains("• #1 · story 12 until 2026-12-04 12:00 UTC"),
        "{list}"
    );
    assert!(
        list.contains("• #2 · component-version pkg:npm/lodash@4.17.20"),
        "{list}"
    );
    assert_eq!(
        instance.send("alice", "/mutes remove 1").await.unwrap(),
        "Mute #1 removed."
    );
    assert_eq!(
        instance.send("alice", "/mutes remove 1").await.unwrap(),
        "There is no mute #1."
    );
}

#[tokio::test]
async fn only_what_changed_since_the_last_notice_is_listed() {
    let instance = Instance::new();
    let mut update = notice();
    // Already exploited when it was last sent; only the fix is new.
    update.previous = Some(Snapshot {
        exploited: true,
        fix_available: false,
        ..update.previous.unwrap()
    });
    let text = render_plain(&instance.conversation.render_notice(&update, "en"));
    assert!(text.contains("• A fix is now available."), "{text}");
    assert!(!text.contains("now known to be exploited"), "{text}");
}

#[tokio::test]
async fn a_story_without_exploitation_signals_says_nothing_about_exploits() {
    let instance = Instance::new();
    let mut new = notice();
    new.kind = NoticeKind::New;
    new.previous = None;
    new.story.exploited = false;
    let text = render_plain(&instance.conversation.render_notice(&new, "en"));
    assert!(!text.to_lowercase().contains("exploit"), "{text}");
}

#[tokio::test]
async fn a_mute_needs_a_positive_duration() {
    let instance = Instance::with_admin().await;
    let reply = instance
        .send("alice", "/mutes add story 12 0d")
        .await
        .unwrap();
    assert!(reply.starts_with("That mute is not valid."), "{reply}");
}
