// SPDX-License-Identifier: AGPL-3.0-or-later

//! Initial scans after uploads, end to end over real SQLite with real OSV records normalized by
//! the OSV source (docs/architecture/inventory.md#upload-flow,
//! docs/architecture/inventory.md#initial-scan-before-the-first-synchronization).

#![allow(clippy::unwrap_used)]

mod common;

use std::sync::{Arc, Mutex};

use common::{Instance, repository_taxonomy};
use jiff::SignedDuration;
use secure_notify_conversation::render_plain;
use secure_notify_core::correlation::Correlator;
use secure_notify_core::delivery::{Notice, Outbox};
use secure_notify_core::ports::SendError;
use secure_notify_core::scan::{Mirror, ScanReport, ScanStore, Scanner};
use secure_notify_core::sources::{
    BACKFILL_DONE, BoxFuture, RawItem, Source, SourceState, SourceStateStore,
};
use secure_notify_core::subscribers::{EndpointRef, SubscriberId};
use secure_notify_source_osv::OsvSource;
use secure_notify_storage::Database;

const ADVISORIES: [&str; 3] = [
    "GHSA-fjxv-7rqg-78g4",
    "GHSA-jchw-25xp-jwwc",
    "GHSA-74fj-2j2h-c42q",
];

/// Records reports instead of sending them.
struct Captured {
    reports: Mutex<Vec<ScanReport>>,
    result: Result<(), SendError>,
}

impl Captured {
    fn new(result: Result<(), SendError>) -> Arc<Self> {
        Arc::new(Self {
            reports: Mutex::new(Vec::new()),
            result,
        })
    }
    fn take(&self) -> Vec<ScanReport> {
        std::mem::take(&mut *self.reports.lock().unwrap())
    }
}

impl Outbox for Captured {
    fn deliver<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move { self.result.clone() })
    }

    fn deliver_report<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        report: &'a ScanReport,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            self.reports.lock().unwrap().push(report.clone());
            self.result.clone()
        })
    }
}

fn osv_mirror() -> Vec<Mirror> {
    vec![Mirror {
        source: "osv".to_string(),
        ecosystems: vec!["npm".to_string()],
    }]
}

fn shrinkwrap() -> Vec<u8> {
    let path = format!(
        "{}/../../plugins/parsers/npm/fixtures/npm-shrinkwrap-v2.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(path).unwrap()
}

/// Ingests the fixture advisories as the OSV mirror's backfill.
async fn backfill(instance: &Instance) {
    let source = OsvSource::new();
    let mut events = Vec::new();
    for id in ADVISORIES {
        let path = format!(
            "{}/tests/fixtures/osv/{id}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let data = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let item = RawItem {
            id: id.to_string(),
            data,
        };
        events.push(source.normalize(&item).unwrap());
    }
    let correlator = Correlator::new(
        instance.database.clone(),
        Arc::new(repository_taxonomy()),
        instance.clock.clone(),
        SignedDuration::from_mins(5),
    );
    correlator
        .ingest_events(true, true, events, true)
        .await
        .unwrap();
}

async fn finish_first_sync(database: &Database) {
    let mut state = SourceState::new();
    state.insert(BACKFILL_DONE.to_string(), "true".to_string());
    database.save_source_state("osv", &state).await.unwrap();
}

fn scanner(instance: &Instance, outbox: Arc<Captured>, mirrors: Vec<Mirror>) -> Scanner<Database> {
    Scanner::new(
        instance.database.clone(),
        outbox,
        mirrors.into(),
        instance.clock.clone(),
    )
}

fn titles(report: &ScanReport) -> Vec<String> {
    report
        .stories
        .iter()
        .map(|(s, _)| s.title.clone())
        .collect()
}

#[tokio::test]
async fn the_scan_waits_for_the_first_synchronization_and_then_reports_once() {
    let instance = Instance::with_mirrors(osv_mirror()).paired().await;
    backfill(&instance).await;
    let outbox = Captured::new(Ok(()));
    let scanner = scanner(&instance, outbox.clone(), osv_mirror());

    let reply = instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    assert!(reply.contains("still synchronizing"), "{reply}");
    assert_eq!(scanner.run_once().await.unwrap(), 0);
    let pending = instance.database.pending_scans().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].1.len(), 8);

    finish_first_sync(&instance.database).await;
    assert_eq!(scanner.run_once().await.unwrap(), 1);
    let reports = outbox.take();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].entries, 8);
    // Most severe first; the advisory fixed before 1.14.9 is not reported.
    assert_eq!(
        titles(&reports[0]),
        [
            "form-data uses unsafe random function in form-data for choosing boundary",
            "Follow Redirects improperly handles URLs in the url.parse() function",
        ]
    );
    for language in ["en", "es"] {
        let text = render_plain(
            &instance
                .conversation
                .render_scan_report(&reports[0], language),
        );
        insta::assert_snapshot!(format!("scan_report_{language}"), text);
    }
    assert!(instance.database.pending_scans().await.unwrap().is_empty());

    // The reported stories are in the ledger: a second upload reports nothing new, at once.
    let reply = instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    assert!(!reply.contains("still synchronizing"), "{reply}");
    assert_eq!(scanner.run_once().await.unwrap(), 1);
    let reports = outbox.take();
    assert!(reports[0].stories.is_empty());
    let none = render_plain(&instance.conversation.render_scan_report(&reports[0], "en"));
    assert_eq!(
        none,
        "Initial scan of 8 entries: no known issues that were not already reported to you."
    );
    let now = *instance.clock.0.lock().unwrap();
    let hits = instance
        .database
        .scan_input(SubscriberId(1), vec!["pkg:npm/form-data".to_string()], now)
        .await
        .unwrap();
    assert!(hits.iter().all(|h| h.candidate.delivered.is_some()));
}

#[tokio::test]
async fn without_a_mirror_in_its_first_synchronization_the_scan_is_immediate() {
    let instance = Instance::with_admin().await;
    backfill(&instance).await;
    let outbox = Captured::new(Ok(()));
    let scanner = scanner(&instance, outbox.clone(), Vec::new());
    let reply = instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    assert!(!reply.contains("still synchronizing"), "{reply}");
    assert_eq!(scanner.run_once().await.unwrap(), 1);
    assert_eq!(outbox.take()[0].stories.len(), 2);
}

#[tokio::test]
async fn a_failed_send_keeps_the_scan_pending() {
    let instance = Instance::with_admin().await;
    let outbox = Captured::new(Err(SendError::Temporary("timeout".to_string())));
    let scanner = scanner(&instance, outbox.clone(), Vec::new());
    instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    assert_eq!(scanner.run_once().await.unwrap(), 0);
    assert_eq!(instance.database.pending_scans().await.unwrap().len(), 1);
}

#[tokio::test]
async fn a_lost_endpoint_deactivates_the_subscriber_and_its_pending_scans() {
    let instance = Instance::with_admin().await;
    let outbox = Captured::new(Err(SendError::EndpointGone));
    let scanner = scanner(&instance, outbox.clone(), Vec::new());
    instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    assert_eq!(scanner.run_once().await.unwrap(), 0);
    assert!(instance.database.pending_scans().await.unwrap().is_empty());
}

#[tokio::test]
async fn exploited_stories_are_marked_in_the_report() {
    let instance = Instance::with_admin().await;
    backfill(&instance).await;
    let outbox = Captured::new(Ok(()));
    let scanner = scanner(&instance, outbox.clone(), Vec::new());
    instance
        .upload("alice", "npm-shrinkwrap.json", shrinkwrap())
        .await
        .unwrap();
    scanner.run_once().await.unwrap();
    let mut report = outbox.take().remove(0);
    report.stories[1].0.exploited = true;
    let text = render_plain(&instance.conversation.render_scan_report(&report, "en"));
    assert!(
        text.contains("• [MEDIUM, exploited] Follow Redirects improperly handles URLs"),
        "{text}"
    );
    assert!(text.contains("• [CRITICAL] form-data"), "{text}");
}
