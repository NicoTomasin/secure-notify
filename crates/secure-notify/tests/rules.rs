// SPDX-License-Identifier: AGPL-3.0-or-later

//! `/rules` through the conversation layer (docs/architecture/matching-and-rules.md#rules,
//! #minimum-severity).

#![allow(clippy::unwrap_used)]

mod common;

use common::Instance;

#[tokio::test]
async fn rules_show_the_defaults_and_set_the_minimum_severity() {
    let instance = Instance::with_admin().await;
    let rules = instance.send("alice", "/rules").await.unwrap();
    assert!(
        rules.starts_with("Your rules (the defaults), in order"),
        "{rules}"
    );
    // The documented table: eight rows, the first notifies exploited stories at any severity.
    assert!(
        rules.contains("• 1. Notify when exploited = true."),
        "{rules}"
    );
    assert!(
        rules.contains(
            "• 3. Notify when relevance = exact-version, if the severity is medium or higher."
        ),
        "{rules}"
    );
    assert!(
        rules.contains(
            "• 8. Notify when relevance = technology, if the severity is high or higher."
        ),
        "{rules}"
    );
    assert!(rules.contains("No minimum severity is set."));

    assert_eq!(
        instance
            .send("alice", "/rules severity HIGH")
            .await
            .unwrap(),
        "Minimum severity set to high. Exploited stories are still notified."
    );
    assert!(
        instance
            .send("alice", "/rules")
            .await
            .unwrap()
            .contains("Minimum severity: high.")
    );
    // `unknown` is a stored level, not a threshold.
    assert!(
        instance
            .send("alice", "/rules severity unknown")
            .await
            .unwrap()
            .contains("is not a level")
    );
    assert_eq!(
        instance.send("alice", "/rules severity off").await.unwrap(),
        "Minimum severity removed."
    );
    assert!(
        instance
            .send("alice", "/rules")
            .await
            .unwrap()
            .contains("No minimum severity is set.")
    );
    assert_eq!(
        instance.send("alice", "/rules reset").await.unwrap(),
        "Your rules were reset to the defaults."
    );
    assert!(
        instance
            .send("alice", "/rules frobnicate")
            .await
            .unwrap()
            .starts_with("Use /rules severity")
    );
}
