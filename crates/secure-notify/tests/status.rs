// SPDX-License-Identifier: AGPL-3.0-or-later

//! `/status` and failure-streak alerts on real SQLite (docs/operations/self-hosting.md#operations).

#![allow(clippy::unwrap_used)]

mod common;

use std::sync::{Arc, Mutex};

use common::Instance;
use secure_notify_conversation::render_plain;
use secure_notify_core::delivery::{Notice, Outbox};
use secure_notify_core::health::{Alerter, SourceAlert, SourceEntry};
use secure_notify_core::ports::SendError;
use secure_notify_core::scan::ScanReport;
use secure_notify_core::sources::{
    BACKFILL_DONE, BoxFuture, FAILURES, ITEMS, LAST_ERROR, LAST_FAILURE, LAST_SUCCESS, SourceState,
    SourceStateStore,
};
use secure_notify_core::subscribers::EndpointRef;

fn source(id: &str, name: &str, enabled: bool) -> SourceEntry {
    SourceEntry {
        id: id.to_string(),
        name: name.to_string(),
        enabled,
        mirror: id == "osv",
        ecosystems: if id == "osv" {
            vec!["npm".to_string()]
        } else {
            Vec::new()
        },
    }
}

async fn instance() -> Instance {
    let instance = Instance::with_sources(vec![
        source("osv", "OSV (npm)", true),
        source("cisa-kev", "CISA Known Exploited Vulnerabilities", true),
        source("nodejs-security", "Node.js security releases", false),
        source("example-feed", "Example feed", true),
    ])
    .paired()
    .await;
    // Counters: three stories from three events, and eight upload entries waiting for a scan.
    instance.backfill_osv_fixtures().await;
    instance
        .upload("alice", "npm-shrinkwrap.json", common::shrinkwrap())
        .await
        .unwrap();
    let save = |id: &'static str, pairs: &'static [(&'static str, &'static str)]| {
        let database = instance.database.clone();
        async move {
            let state: SourceState = pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect();
            database.save_source_state(id, &state).await.unwrap();
        }
    };
    save(
        "osv",
        &[
            (BACKFILL_DONE, "true"),
            (LAST_SUCCESS, "2026-11-27T11:00:00Z"),
            (FAILURES, "0"),
            (ITEMS, "232471"),
        ],
    )
    .await;
    save(
        "cisa-kev",
        &[
            (BACKFILL_DONE, "true"),
            (LAST_SUCCESS, "2026-11-27T08:00:00Z"),
            (LAST_FAILURE, "2026-11-27T11:30:00Z"),
            (LAST_ERROR, "unexpected response: HTTP 503"),
            (FAILURES, "3"),
            (ITEMS, "1808"),
        ],
    )
    .await;
    instance
}

#[tokio::test]
async fn status_shows_each_source_and_the_instance_counters_to_admins() {
    let instance = instance().await;
    let code = instance.invite("alice").await;
    instance
        .send("bob", &format!("/start {code}"))
        .await
        .unwrap();
    let denied = instance.send("bob", "/status").await.unwrap();
    assert!(!denied.contains("Instance status"), "{denied}");

    let english = instance.send("alice", "/status").await.unwrap();
    insta::assert_snapshot!("status_en", english);
    instance.send("alice", "/language es").await.unwrap();
    let spanish = instance.send("alice", "/status").await.unwrap();
    insta::assert_snapshot!("status_es", spanish);
}

/// Records alerts per address.
#[derive(Default)]
struct Sent(Mutex<Vec<(String, String)>>);

impl Outbox for Sent {
    fn deliver<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async { Ok(()) })
    }
    fn deliver_report<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a ScanReport,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async { Ok(()) })
    }
    fn deliver_alert<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        _: &'a SourceAlert,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            self.0
                .lock()
                .unwrap()
                .push((endpoint.address.clone(), language.to_string()));
            Ok(())
        })
    }
}

#[tokio::test]
async fn failure_streak_alerts_reach_admins_only() {
    let instance = instance().await;
    let code = instance.invite("alice").await;
    instance
        .send("bob", &format!("/start {code}"))
        .await
        .unwrap();
    let outbox = Arc::new(Sent::default());
    let alert = SourceAlert {
        source: "CISA Known Exploited Vulnerabilities".to_string(),
        failures: 3,
        error: "unexpected response: HTTP 503".to_string(),
    };
    let reached = Alerter::new(instance.database.clone(), outbox.clone())
        .alert(&alert)
        .await
        .unwrap();
    assert_eq!(reached, 1);
    assert_eq!(
        *outbox.0.lock().unwrap(),
        [("alice".to_string(), "en".to_string())]
    );
    for language in ["en", "es"] {
        let text = render_plain(&instance.conversation.render_alert(&alert, language));
        insta::assert_snapshot!(format!("alert_{language}"), text);
    }
}
