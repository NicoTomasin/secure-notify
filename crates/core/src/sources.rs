// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sources: the contract, the normalizer, and the runner that keeps state, backfills on the first
//! synchronization, tracks health, and backs off on failures
//! (docs/architecture/plugins.md#sources, docs/architecture/deduplication.md#backfill).

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;

use jiff::{SignedDuration, Timestamp};
use secure_notify_model::NormalizedEvent;

use crate::ports::{HttpError, HttpRequest, HttpResponse, StoreError};

/// A boxed future, for the object-safe source contract.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Persistent key/value state of a source: its cursors, plus the runner's keys under `sync.` and
/// `health.`.
pub type SourceState = BTreeMap<String, String>;

/// State key set once the first synchronization (the backfill) succeeded.
pub const BACKFILL_DONE: &str = "sync.backfill_done";
/// Health keys (docs/operations/self-hosting.md#operations).
pub const LAST_SUCCESS: &str = "health.last_success";
/// Time of the last failed run.
pub const LAST_FAILURE: &str = "health.last_failure";
/// Message of the last failure.
pub const LAST_ERROR: &str = "health.last_error";
/// Time of the last attempt, successful or not.
pub const LAST_ATTEMPT: &str = "health.last_attempt";
/// Consecutive failed runs.
pub const FAILURES: &str = "health.failures";
/// Events accepted since the source was first run.
pub const ITEMS: &str = "health.items";

/// Longest delay between attempts while a source keeps failing.
pub const MAX_BACKOFF: SignedDuration = SignedDuration::from_hours(24);

/// One raw item, as fetched; `normalize` turns it into one event.
#[derive(Debug, Clone, PartialEq)]
pub struct RawItem {
    /// The item's ID within the source, used in logs.
    pub id: String,
    /// The item's data.
    pub data: serde_json::Value,
}

/// What the runner needs to know about a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInfo {
    /// Source ID; the `source` of its events.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Time between runs.
    pub interval: SignedDuration,
    /// Whether the source mirrors a whole database (exempt from retention).
    pub mirror: bool,
    /// Whether it is enabled when the operator says nothing.
    pub default_enabled: bool,
    /// The only hosts it may contact.
    pub hosts: Vec<String>,
}

/// HTTP as seen by a source: the shared client, restricted to the source's declared hosts.
pub trait SourceHttp: Send + Sync {
    /// Performs a `GET` request.
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>>;
}

/// Why a fetch failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The HTTP request failed.
    #[error("{0}")]
    Http(#[from] HttpError),
    /// The source answered something unexpected.
    #[error("unexpected response: {0}")]
    Response(String),
}

/// The source contract (docs/architecture/plugins.md#sources).
pub trait Source: Send + Sync {
    /// Source information.
    fn info(&self) -> &SourceInfo;

    /// Fetches raw items, incrementally, keeping cursors in `state`.
    fn fetch<'a>(
        &'a self,
        http: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<Vec<RawItem>, SourceError>>;

    /// Turns one raw item into one event; a pure function of the item.
    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, String>;
}

/// Persistence for source state.
pub trait SourceStateStore: Send + Sync {
    /// The state of a source (empty if it never ran).
    fn source_state(
        &self,
        source: &str,
    ) -> impl Future<Output = Result<SourceState, StoreError>> + Send;

    /// Replaces the state of a source.
    fn save_source_state(
        &self,
        source: &str,
        state: &SourceState,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// Where validated events go (the correlator).
pub trait EventSink: Send + Sync {
    /// Ingests events of one run. Returns how many were new or changed.
    fn ingest(
        &self,
        source: &SourceInfo,
        events: Vec<NormalizedEvent>,
        backfill: bool,
    ) -> impl Future<Output = Result<usize, StoreError>> + Send;
}

/// Normalizes and validates raw items. Invalid items are dropped and logged; they never stop the
/// run (docs/architecture/plugins.md#sources). Returns the valid events and the number dropped.
pub fn normalize_items(source: &dyn Source, items: &[RawItem]) -> (Vec<NormalizedEvent>, usize) {
    let id = &source.info().id;
    let mut events = Vec::with_capacity(items.len());
    let mut dropped = 0;
    for item in items {
        let event = source.normalize(item).and_then(|event| {
            if &event.source != id {
                return Err(format!("event source `{}` is not `{id}`", event.source));
            }
            event.validate().map_err(|e| e.to_string())?;
            Ok(event)
        });
        match event {
            Ok(event) => events.push(event),
            Err(reason) => {
                dropped += 1;
                tracing::warn!(source = %id, item = %item.id, reason = %reason, "item dropped");
            }
        }
    }
    (events, dropped)
}

/// When a source should run next: one interval after the last attempt, doubled for every
/// consecutive failure up to [`MAX_BACKOFF`]. A source that never ran is due now.
pub fn next_run(info: &SourceInfo, state: &SourceState, now: Timestamp) -> Timestamp {
    let Some(last) = state
        .get(LAST_ATTEMPT)
        .and_then(|t| t.parse::<Timestamp>().ok())
    else {
        return now;
    };
    let failures: u32 = state
        .get(FAILURES)
        .and_then(|f| f.parse().ok())
        .unwrap_or(0);
    let factor = 2_i32.saturating_pow(failures.min(16));
    let delay = info
        .interval
        .checked_mul(factor)
        .map_or(MAX_BACKOFF, |d| d.min(MAX_BACKOFF));
    let delay = if failures == 0 { info.interval } else { delay };
    last.checked_add(delay).unwrap_or(now)
}

/// Result of one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    /// Whether the run was the backfill.
    pub backfill: bool,
    /// Items fetched.
    pub fetched: usize,
    /// Items dropped by normalization or validation.
    pub dropped: usize,
    /// Events the sink accepted as new or changed.
    pub ingested: usize,
    /// The failure, if the run failed.
    pub error: Option<String>,
}

/// Runs a source once: fetch, normalize, validate, ingest, and record state and health. A failure
/// is recorded and reported, never propagated to other sources.
pub async fn run_once<St, Sk>(
    source: &dyn Source,
    http: &dyn SourceHttp,
    store: &St,
    sink: &Sk,
    now: Timestamp,
) -> Result<RunReport, StoreError>
where
    St: SourceStateStore,
    Sk: EventSink,
{
    let info = source.info();
    let mut state = store.source_state(&info.id).await?;
    let backfill = !state.contains_key(BACKFILL_DONE);
    let mut cursors = state.clone();
    let outcome = source.fetch(http, &mut cursors).await;
    // The runner's own keys come from `state`; cursors only from a successful fetch.
    let mut report = RunReport {
        backfill,
        fetched: 0,
        dropped: 0,
        ingested: 0,
        error: None,
    };
    state.insert(LAST_ATTEMPT.to_string(), now.to_string());
    match outcome {
        Ok(items) => {
            let (events, dropped) = normalize_items(source, &items);
            report.fetched = items.len();
            report.dropped = dropped;
            report.ingested = sink.ingest(info, events, backfill).await?;
            for (key, value) in cursors {
                if !key.starts_with("sync.") && !key.starts_with("health.") {
                    state.insert(key, value);
                }
            }
            let total: usize = state.get(ITEMS).and_then(|n| n.parse().ok()).unwrap_or(0);
            state.insert(ITEMS.to_string(), (total + report.ingested).to_string());
            state.insert(LAST_SUCCESS.to_string(), now.to_string());
            state.insert(FAILURES.to_string(), "0".to_string());
            state.insert(BACKFILL_DONE.to_string(), "true".to_string());
        }
        Err(error) => {
            let failures: u32 = state
                .get(FAILURES)
                .and_then(|f| f.parse().ok())
                .unwrap_or(0);
            state.insert(FAILURES.to_string(), (failures + 1).to_string());
            state.insert(LAST_FAILURE.to_string(), now.to_string());
            state.insert(LAST_ERROR.to_string(), error.to_string());
            report.error = Some(error.to_string());
        }
    }
    store.save_source_state(&info.id, &state).await?;
    Ok(report)
}
