// SPDX-License-Identifier: AGPL-3.0-or-later

//! Initial scans after uploads (docs/architecture/inventory.md#upload-flow,
//! docs/architecture/inventory.md#initial-scan-before-the-first-synchronization).
//!
//! An upload records its entries as pending scans. The scanner checks the entries whose ecosystem
//! has no mirror still in its first synchronization against local data, backfilled stories
//! included, sends one report, and records the reported stories in the delivery ledger.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;

use jiff::Timestamp;

use crate::correlation::StoryId;
use crate::delivery::{DeliveryStore, Outbox, StoryView};
use crate::inventory::ParsedComponent;
use crate::matching::{
    Candidate, Reason, Relevance, Snapshot, StoryFacts, decide, default_rules, relevance,
};
use crate::ports::{Clock, SendError, StoreError};
use crate::severity;
use crate::sources::{BACKFILL_DONE, SourceStateStore};
use crate::subscribers::{EndpointRef, SubscriberId};

/// An enabled mirror source and the purl types it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mirror {
    /// Source ID.
    pub source: String,
    /// Purl types (`npm`).
    pub ecosystems: Vec<String>,
}

/// The purl type of a component (`npm` for `pkg:npm/lodash`).
pub fn purl_type(component: &str) -> Option<&str> {
    component.strip_prefix("pkg:")?.split('/').next()
}

/// Purl types covered by a mirror that has not completed its first synchronization.
pub async fn syncing_ecosystems<S: SourceStateStore>(
    store: &S,
    mirrors: &[Mirror],
) -> Result<BTreeSet<String>, StoreError> {
    let mut syncing = BTreeSet::new();
    for mirror in mirrors {
        if !store
            .source_state(&mirror.source)
            .await?
            .contains_key(BACKFILL_DONE)
        {
            syncing.extend(mirror.ecosystems.iter().cloned());
        }
    }
    Ok(syncing)
}

/// Whether an entry must wait for a mirror's first synchronization.
pub fn waits(entry: &ParsedComponent, syncing: &BTreeSet<String>) -> bool {
    purl_type(&entry.purl).is_some_and(|t| syncing.contains(t))
}

/// A story affecting scanned components, with the subscriber's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanHit {
    /// The story as shown.
    pub view: StoryView,
    /// The story's facts.
    pub facts: StoryFacts,
    /// The subscriber as a candidate of the story.
    pub candidate: Candidate,
}

/// The report of one initial scan.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanReport {
    /// Entries scanned.
    pub entries: usize,
    /// Stories that affect them, most severe first.
    pub stories: Vec<(StoryView, Reason)>,
}

/// Persistence for initial scans.
pub trait ScanStore: Send + Sync {
    /// Records entries of an upload as pending scans.
    fn add_pending_scans(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Pending scans of active subscribers, grouped by subscriber.
    fn pending_scans(
        &self,
    ) -> impl Future<Output = Result<Vec<(SubscriberId, Vec<ParsedComponent>)>, StoreError>> + Send;

    /// Stories affecting any of `components`, with the subscriber as their candidate.
    fn scan_input(
        &self,
        subscriber: SubscriberId,
        components: Vec<String>,
        now: Timestamp,
    ) -> impl Future<Output = Result<Vec<ScanHit>, StoreError>> + Send;

    /// The endpoint reports go to and the subscriber's language.
    fn scan_recipient(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<(Option<(i64, EndpointRef)>, String), StoreError>> + Send;

    /// Records the reported stories in the ledger (as `scan`, skipping stories already in it) and
    /// removes the pending scans, atomically.
    fn complete_scan(
        &self,
        subscriber: SubscriberId,
        entries: Vec<ParsedComponent>,
        reported: Vec<(StoryId, Snapshot)>,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// What one scan reports: stories not yet delivered whose relevance comes from the scanned
/// entries and that pass the subscriber's rules, most severe and exploited first.
pub fn report(entries: &[ParsedComponent], hits: Vec<ScanHit>) -> ScanReport {
    let defaults = default_rules();
    let pairs: Vec<(String, Option<String>)> = entries
        .iter()
        .map(|e| (e.purl.clone(), e.version.clone()))
        .collect();
    let mut stories = Vec::new();
    for mut hit in hits {
        if hit.candidate.delivered.is_some() {
            continue;
        }
        // Only the scanned entries count; watches are matched by the regular pipeline.
        hit.candidate.entries.clone_from(&pairs);
        let Some(reason) = relevance(hit.view.id, &hit.facts, &hit.candidate) else {
            continue;
        };
        if !matches!(
            reason.relevance,
            Relevance::ExactVersion | Relevance::Component
        ) {
            continue;
        }
        let rules = if hit.candidate.rules.is_empty() {
            &defaults
        } else {
            &hit.candidate.rules
        };
        if decide(rules, hit.candidate.min_severity, &hit.facts, &reason) {
            stories.push((hit.view, reason));
        }
    }
    stories.sort_by(|(a, _), (b, _)| {
        (b.exploited, severity::rank(b.severity), b.id.0).cmp(&(
            a.exploited,
            severity::rank(a.severity),
            a.id.0,
        ))
    });
    ScanReport {
        entries: entries.len(),
        stories,
    }
}

/// Runs pending initial scans.
pub struct Scanner<S> {
    store: S,
    outbox: Arc<dyn Outbox>,
    mirrors: Arc<[Mirror]>,
    clock: Arc<dyn Clock>,
}

impl<S: ScanStore + DeliveryStore + SourceStateStore> Scanner<S> {
    /// Creates the scanner.
    pub fn new(
        store: S,
        outbox: Arc<dyn Outbox>,
        mirrors: Arc<[Mirror]>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            outbox,
            mirrors,
            clock,
        }
    }

    /// Scans every pending entry whose mirrors are ready. Returns how many reports were sent.
    pub async fn run_once(&self) -> Result<usize, StoreError> {
        let syncing = syncing_ecosystems(&self.store, &self.mirrors).await?;
        let now = self.clock.now();
        let mut sent = 0;
        for (subscriber, entries) in self.store.pending_scans().await? {
            let ready: Vec<ParsedComponent> = entries
                .into_iter()
                .filter(|e| !waits(e, &syncing))
                .collect();
            if ready.is_empty() {
                continue;
            }
            let (endpoint, language) = self.store.scan_recipient(subscriber).await?;
            let Some((endpoint_id, endpoint)) = endpoint else {
                self.store
                    .complete_scan(subscriber, ready, Vec::new(), now)
                    .await?;
                continue;
            };
            let components: Vec<String> = ready
                .iter()
                .map(|e| e.purl.clone())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let hits = self.store.scan_input(subscriber, components, now).await?;
            let facts: Vec<(StoryId, Snapshot)> = hits
                .iter()
                .map(|h| (h.view.id, h.facts.snapshot()))
                .collect();
            let report = report(&ready, hits);
            let reported = facts
                .into_iter()
                .filter(|(id, _)| report.stories.iter().any(|(s, _)| s.id == *id))
                .collect();
            match self
                .outbox
                .deliver_report(&endpoint, &language, &report)
                .await
            {
                Ok(()) => {
                    self.store
                        .complete_scan(subscriber, ready, reported, now)
                        .await?;
                    sent += 1;
                }
                Err(SendError::EndpointGone) => self.store.endpoint_gone(endpoint_id).await?,
                Err(SendError::Temporary(error)) => {
                    // The pending scans stay and are retried on the next run.
                    tracing::warn!(error = %error, "initial scan report not sent");
                }
            }
        }
        Ok(sent)
    }
}
