// SPDX-License-Identifier: AGPL-3.0-or-later

//! The source runner with fakes at its ports (docs/architecture/plugins.md#sources,
//! docs/architecture/deduplication.md#backfill).

// The fakes implement async ports synchronously.
#![allow(clippy::unwrap_used, clippy::unused_async_trait_impl)]

use std::collections::HashMap;
use std::sync::Mutex;

use jiff::{SignedDuration, Timestamp};
use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse, StoreError};
use secure_notify_core::sources::{
    BACKFILL_DONE, BoxFuture, Download, EventSink, FAILURES, FetchOutput, LAST_ERROR, RawItem,
    Source, SourceError, SourceHttp, SourceInfo, SourceState, SourceStateStore, next_run, run_once,
};
use secure_notify_model::NormalizedEvent;
use serde_json::json;

struct NoHttp;

impl SourceHttp for NoHttp {
    fn get(&self, _: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async { Err(HttpError::Refused("no network in tests".to_string())) })
    }

    fn download(&self, _: HttpRequest, _: u64) -> BoxFuture<'_, Result<Download, HttpError>> {
        Box::pin(async { Err(HttpError::Refused("no network in tests".to_string())) })
    }
}

/// Yields the configured items and moves a cursor, or fails.
struct FakeSource {
    info: SourceInfo,
    items: Vec<RawItem>,
    fail: bool,
    withdrawn: Vec<String>,
}

impl Source for FakeSource {
    fn info(&self) -> &SourceInfo {
        &self.info
    }

    fn fetch<'a>(
        &'a self,
        _: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<FetchOutput, SourceError>> {
        Box::pin(async move {
            state.insert("cursor".to_string(), "next".to_string());
            if self.fail {
                Err(SourceError::Response("HTTP 503".to_string()))
            } else {
                let mut output = FetchOutput::items(self.items.clone());
                output.withdrawn.clone_from(&self.withdrawn);
                Ok(output)
            }
        })
    }

    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, String> {
        serde_json::from_value(item.data.clone()).map_err(|e| e.to_string())
    }
}

#[derive(Default)]
struct Memory {
    state: Mutex<HashMap<String, SourceState>>,
    ingested: Mutex<Vec<(usize, bool)>>,
    withdrawn: Mutex<Vec<String>>,
}

impl SourceStateStore for Memory {
    async fn source_state(&self, source: &str) -> Result<SourceState, StoreError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .get(source)
            .cloned()
            .unwrap_or_default())
    }

    async fn save_source_state(&self, source: &str, state: &SourceState) -> Result<(), StoreError> {
        self.state
            .lock()
            .unwrap()
            .insert(source.to_string(), state.clone());
        Ok(())
    }
}

impl EventSink for Memory {
    async fn ingest(
        &self,
        _: &SourceInfo,
        events: Vec<NormalizedEvent>,
        backfill: bool,
    ) -> Result<usize, StoreError> {
        self.ingested.lock().unwrap().push((events.len(), backfill));
        Ok(events.len())
    }

    async fn withdraw(&self, _: &SourceInfo, items: Vec<String>) -> Result<usize, StoreError> {
        let count = items.len();
        self.withdrawn.lock().unwrap().extend(items);
        Ok(count)
    }
}

fn info() -> SourceInfo {
    SourceInfo {
        id: "fake".to_string(),
        name: "Fake".to_string(),
        interval: SignedDuration::from_mins(30),
        mirror: false,
        ecosystems: Vec::new(),
        default_enabled: true,
        structured: false,
        hosts: Vec::new(),
    }
}

fn item(source: &str, id: &str) -> RawItem {
    RawItem {
        id: id.to_string(),
        data: json!({
            "schema_version": "1", "source": source, "source_item_id": id,
            "title": "Example advisory", "kind": "vulnerability"
        }),
    }
}

fn now(text: &str) -> Timestamp {
    text.parse().unwrap()
}

#[tokio::test]
async fn the_first_successful_run_is_a_backfill_and_later_runs_are_not() {
    let source = FakeSource {
        info: info(),
        items: vec![item("fake", "A-1")],
        fail: false,
        withdrawn: Vec::new(),
    };
    let memory = Memory::default();
    let first = run_once(
        &source,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T10:00:00Z"),
    )
    .await
    .unwrap();
    let second = run_once(
        &source,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T10:30:00Z"),
    )
    .await
    .unwrap();
    assert!(first.backfill);
    assert!(!second.backfill);
    assert_eq!(*memory.ingested.lock().unwrap(), [(1, true), (1, false)]);
}

#[tokio::test]
async fn a_failed_first_run_keeps_the_backfill_pending_and_no_cursor() {
    let failing = FakeSource {
        info: info(),
        items: Vec::new(),
        fail: true,
        withdrawn: Vec::new(),
    };
    let memory = Memory::default();
    let report = run_once(
        &failing,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T10:00:00Z"),
    )
    .await
    .unwrap();
    assert_eq!(
        report.error.as_deref(),
        Some("unexpected response: HTTP 503")
    );
    let state = memory.state.lock().unwrap()["fake"].clone();
    assert_eq!(state.get(FAILURES).map(String::as_str), Some("1"));
    assert_eq!(
        state.get(LAST_ERROR).map(String::as_str),
        Some("unexpected response: HTTP 503")
    );
    assert!(
        !state.contains_key("cursor"),
        "a failed fetch must not move the cursor"
    );
    assert!(!state.contains_key(BACKFILL_DONE));
    // The next successful run is still the backfill.
    let working = FakeSource {
        info: info(),
        items: vec![item("fake", "A-1")],
        fail: false,
        withdrawn: Vec::new(),
    };
    let report = run_once(
        &working,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T11:00:00Z"),
    )
    .await
    .unwrap();
    assert!(report.backfill);
}

#[tokio::test]
async fn invalid_items_are_dropped_without_stopping_the_run() {
    // plugins.md: invalid events are dropped and logged, and never crash the pipeline.
    let mut bad = item("fake", "A-2");
    bad.data["kind"] = json!("rumor");
    let items = vec![item("fake", "A-1"), bad, item("other-source", "A-3")];
    let source = FakeSource {
        info: info(),
        items,
        fail: false,
        withdrawn: Vec::new(),
    };
    let memory = Memory::default();
    let report = run_once(
        &source,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T10:00:00Z"),
    )
    .await
    .unwrap();
    assert_eq!((report.fetched, report.dropped, report.ingested), (3, 2, 1));
}

#[test]
fn failing_sources_back_off_exponentially_up_to_a_day() {
    let info = info();
    let last = "2026-11-27T10:00:00Z";
    let mut state = SourceState::new();
    assert_eq!(
        next_run(&info, &state, now(last)),
        now(last),
        "never ran: due now"
    );
    state.insert("health.last_attempt".to_string(), last.to_string());
    let cases = [
        ("0", "2026-11-27T10:30:00Z"),
        ("1", "2026-11-27T11:00:00Z"),
        ("2", "2026-11-27T12:00:00Z"),
        ("10", "2026-11-28T10:00:00Z"),
    ];
    for (failures, expected) in cases {
        state.insert(FAILURES.to_string(), failures.to_string());
        assert_eq!(
            next_run(&info, &state, now(last)),
            now(expected),
            "{failures} failures"
        );
    }
}

#[tokio::test]
async fn withdrawn_items_reported_by_a_source_are_removed() {
    let source = FakeSource {
        info: info(),
        items: Vec::new(),
        fail: false,
        withdrawn: vec!["A-1".to_string()],
    };
    let memory = Memory::default();
    run_once(
        &source,
        &NoHttp,
        &memory,
        &memory,
        now("2026-11-27T10:00:00Z"),
    )
    .await
    .unwrap();
    assert_eq!(*memory.withdrawn.lock().unwrap(), ["A-1"]);
}
