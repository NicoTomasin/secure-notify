// SPDX-License-Identifier: AGPL-3.0-or-later

//! Event-to-story correlation (docs/architecture/deduplication.md).
//!
//! Signals, in order: identifiers (with aliases), then linked identifiers found in the event's
//! text and links, then the canonical URL. Events correlate only with stories of a compatible kind
//! family; an identifier shared with a story of the other family links the stories as related.
//! Explicit identifiers belonging to several stories merge them. The logic runs inside one write
//! transaction through [`CorrelationTx`], which storage implements.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;

use jiff::{SignedDuration, Timestamp};
use secure_notify_model::{
    Affected, EventKind, KindFamily, NormalizedEvent, RangeEvent, SeverityLevel,
};

use crate::identifiers;
use crate::ports::{Clock, StoreError};
use crate::severity;
use crate::sources::{EventSink, SourceInfo};
use crate::taxonomy::Taxonomy;

/// Internal story ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StoryId(pub i64);

/// Events per write transaction, so a large backfill does not hold the writer for long.
const BATCH: usize = 500;

/// An event already stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEvent {
    /// Row ID.
    pub id: i64,
    /// Its story.
    pub story: StoryId,
    /// Hash of its normalized content.
    pub content_hash: String,
}

/// An event of a story, as needed to aggregate it.
#[derive(Debug, Clone, PartialEq)]
pub struct EventRecord {
    /// Normalized event; `None` once detailed data expired.
    pub payload: Option<NormalizedEvent>,
    /// Whether it came from a backfill.
    pub backfill: bool,
    /// Title priority of its source.
    pub title_priority: i64,
}

/// How an event entered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMeta {
    /// From a backfill.
    pub backfill: bool,
    /// From a mirror source (exempt from retention).
    pub mirror: bool,
    /// Title priority of its source.
    pub title_priority: i64,
    /// When it was fetched.
    pub fetched_at: Timestamp,
}

/// The stored fields of a story.
#[derive(Debug, Clone, PartialEq)]
pub struct StoryRecord {
    /// Kind.
    pub kind: EventKind,
    /// Title.
    pub title: String,
    /// Priority of the source the title came from.
    pub title_priority: i64,
    /// Summary.
    pub summary: Option<String>,
    /// URL to show.
    pub url: Option<String>,
    /// Earliest publication time.
    pub published_at: Option<Timestamp>,
    /// Highest level among its events.
    pub severity: SeverityLevel,
    /// Preferred CVSS (version, score, vector).
    pub cvss: Option<(String, f64, Option<String>)>,
    /// Latest EPSS (score, percentile).
    pub epss: Option<(f64, f64)>,
    /// Known to be exploited.
    pub exploited: bool,
    /// Public exploit available.
    pub exploit_available: bool,
    /// A fix is available.
    pub fix_available: bool,
    /// When the story was created.
    pub first_seen_at: Timestamp,
    /// Last change.
    pub updated_at: Timestamp,
    /// Grace window end.
    pub eligible_at: Option<Timestamp>,
    /// The matcher must process the story.
    pub needs_match: bool,
    /// Every event is backfilled.
    pub backfill: bool,
    /// Minimal deduplication record.
    pub compacted: bool,
}

/// What one ingestion did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestCounts {
    /// New events.
    pub new: usize,
    /// Events whose content changed.
    pub changed: usize,
    /// Events fetched again without change.
    pub unchanged: usize,
}

/// Operations inside the correlation transaction.
pub trait CorrelationTx {
    /// The stored event `(source, item)`.
    fn find_event(&mut self, source: &str, item: &str) -> Result<Option<StoredEvent>, StoreError>;
    /// The story holding identifier `(kind, value)`.
    fn story_by_identifier(
        &mut self,
        kind: &str,
        value: &str,
    ) -> Result<Option<StoryId>, StoreError>;
    /// A story.
    fn story(&mut self, id: StoryId) -> Result<StoryRecord, StoreError>;
    /// The events of a story, oldest first.
    fn story_events(&mut self, id: StoryId) -> Result<Vec<EventRecord>, StoreError>;
    /// Creates a story.
    fn insert_story(&mut self, record: &StoryRecord) -> Result<StoryId, StoreError>;
    /// Replaces a story's fields.
    fn update_story(&mut self, id: StoryId, record: &StoryRecord) -> Result<(), StoreError>;
    /// Stores a new event.
    fn insert_event(
        &mut self,
        story: StoryId,
        event: &NormalizedEvent,
        hash: &str,
        meta: &EventMeta,
    ) -> Result<(), StoreError>;
    /// Replaces the content of a stored event.
    fn update_event(
        &mut self,
        id: i64,
        event: &NormalizedEvent,
        hash: &str,
        meta: &EventMeta,
    ) -> Result<(), StoreError>;
    /// Adds identifiers to a story; identifiers held by another story are left there.
    fn add_identifiers(
        &mut self,
        story: StoryId,
        ids: &[(String, String)],
    ) -> Result<(), StoreError>;
    /// Replaces a story's affected components.
    fn replace_affected(&mut self, story: StoryId, affected: &[Affected])
    -> Result<(), StoreError>;
    /// Replaces a story's entities, with their origin.
    fn replace_entities(
        &mut self,
        story: StoryId,
        source: &BTreeSet<String>,
        core: &BTreeSet<String>,
    ) -> Result<(), StoreError>;
    /// Links two stories of different families.
    fn add_relation(&mut self, a: StoryId, b: StoryId) -> Result<(), StoreError>;
    /// Merges `from` into `into`: events, identifiers, relations, ledger, queue, and story mutes
    /// move; `from` is recorded in `story_merges` and deleted.
    fn merge_stories(&mut self, from: StoryId, into: StoryId) -> Result<(), StoreError>;
}

/// The unit of work that runs correlation in one transaction.
pub type CorrelationWork =
    Box<dyn FnOnce(&mut dyn CorrelationTx) -> Result<IngestCounts, StoreError> + Send>;

/// Persistence for correlation.
pub trait CorrelationStore: Send + Sync {
    /// Runs `work` in one write transaction.
    fn correlate(
        &self,
        work: CorrelationWork,
    ) -> impl Future<Output = Result<IngestCounts, StoreError>> + Send;
}

/// Hash of an event's normalized content (64-bit FNV-1a over its JSON), to detect changes.
pub fn content_hash(event: &NormalizedEvent) -> String {
    let json = serde_json::to_string(event).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in json.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// The canonical form of a URL: lowercase scheme and host, no fragment, no tracking parameters
/// (`utm_*`, `fbclid`, `gclid`), no trailing slash (docs/architecture/deduplication.md).
pub fn canonical_url(url: &str) -> Option<String> {
    let mut parsed = url::Url::parse(url.trim()).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    parsed.set_fragment(None);
    let kept: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(k, _)| !(k.starts_with("utm_") || k == "fbclid" || k == "gclid"))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        parsed.set_query(None);
    } else {
        parsed.query_pairs_mut().clear().extend_pairs(kept);
    }
    let path = parsed.path().to_string();
    if path.len() > 1 && path.ends_with('/') {
        parsed.set_path(path.trim_end_matches('/'));
    }
    let mut text = parsed.to_string();
    if parsed.path() == "/" && parsed.query().is_none() {
        text = text.trim_end_matches('/').to_string();
    }
    Some(text)
}

/// Title priority of an event: structured databases before feeds before news.
pub fn title_priority(structured: bool, kind: EventKind) -> i64 {
    match (kind, structured) {
        (EventKind::News, _) => 0,
        (_, true) => 2,
        (_, false) => 1,
    }
}

fn compatible(a: KindFamily, b: KindFamily) -> bool {
    a == b || a == KindFamily::Both || b == KindFamily::Both
}

/// Explicit identifiers of an event as `(kind, value)`.
fn explicit_identifiers(event: &NormalizedEvent) -> Vec<(String, String)> {
    event
        .identifiers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Identifiers found in an event's title, summary, and links, not already explicit.
fn linked_identifiers(event: &NormalizedEvent) -> Vec<(String, String)> {
    let text = format!(
        "{} {} {}",
        event.title,
        event.summary.as_deref().unwrap_or_default(),
        event.links.join(" ")
    );
    let (cves, ghsas) = identifiers::extract(&text);
    let explicit = explicit_identifiers(event);
    cves.into_iter()
        .map(|v| ("cve".to_string(), v))
        .chain(ghsas.into_iter().map(|v| ("ghsa".to_string(), v)))
        .filter(|pair| !explicit.contains(pair))
        .collect()
}

fn precedence(kind: EventKind) -> u8 {
    match kind {
        EventKind::MaliciousPackage | EventKind::Breach => 6,
        EventKind::Vulnerability | EventKind::Attack => 5,
        EventKind::Bug | EventKind::Outage => 4,
        EventKind::Release => 3,
        EventKind::Exploit => 2,
        EventKind::News => 0,
    }
}

/// The aggregate of a story's events (docs/architecture/data-model.md#story).
struct Aggregate {
    kind: EventKind,
    title: String,
    title_priority: i64,
    summary: Option<String>,
    url: Option<String>,
    published_at: Option<Timestamp>,
    severity: SeverityLevel,
    cvss: Option<(String, f64, Option<String>)>,
    epss: Option<(f64, f64)>,
    exploited: bool,
    exploit_available: bool,
    fix_available: bool,
    affected: Vec<Affected>,
    identifiers: Vec<(String, String)>,
    source_entities: BTreeSet<String>,
    core_entities: BTreeSet<String>,
    backfill: bool,
}

fn cvss_rank(version: &str) -> u8 {
    match version {
        "4.0" => 4,
        "3.1" => 3,
        "3.0" => 2,
        _ => 1,
    }
}

fn aggregate(events: &[EventRecord], taxonomy: &Taxonomy) -> Option<Aggregate> {
    let with_payload: Vec<(&NormalizedEvent, i64)> = events
        .iter()
        .filter_map(|e| e.payload.as_ref().map(|p| (p, e.title_priority)))
        .collect();
    let (first, _) = with_payload.first()?;
    let kind = with_payload
        .iter()
        .map(|(e, _)| e.kind)
        .max_by_key(|k| precedence(*k))
        .unwrap_or(first.kind);
    let (lead, lead_priority) = with_payload
        .iter()
        .rev()
        .max_by_key(|(_, p)| *p)
        .copied()
        .unwrap_or((first, 0));
    let mut aggregate = Aggregate {
        kind,
        title: lead.title.clone(),
        title_priority: lead_priority,
        summary: lead.summary.clone(),
        url: lead.url.clone(),
        published_at: None,
        severity: SeverityLevel::Unknown,
        cvss: None,
        epss: None,
        exploited: false,
        exploit_available: false,
        fix_available: false,
        affected: Vec::new(),
        identifiers: Vec::new(),
        source_entities: BTreeSet::new(),
        core_entities: BTreeSet::new(),
        backfill: events.iter().all(|e| e.backfill),
    };
    for (event, _) in &with_payload {
        aggregate.summary = aggregate.summary.or_else(|| event.summary.clone());
        aggregate.url = aggregate.url.or_else(|| event.url.clone());
        aggregate.published_at = match (aggregate.published_at, event.published_at) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        aggregate.severity = severity::max(aggregate.severity, severity::normalize(event));
        if let Some(cvss) = &event.severity.cvss {
            let better = aggregate
                .cvss
                .as_ref()
                .is_none_or(|(v, _, _)| cvss_rank(&cvss.version) > cvss_rank(v));
            if better {
                aggregate.cvss = Some((cvss.version.clone(), cvss.score, cvss.vector.clone()));
            }
        }
        if let Some(epss) = &event.severity.epss {
            aggregate.epss = Some((epss.score, epss.percentile));
        }
        aggregate.exploited |= event.severity.exploited;
        aggregate.exploit_available |= event.severity.exploit_available;
        for affected in &event.affected {
            let fixed = !affected.fixed.is_empty()
                || affected
                    .ranges
                    .iter()
                    .any(|r| r.events.iter().any(|e| matches!(e, RangeEvent::Fixed(_))));
            aggregate.fix_available |= fixed;
            let key = &affected.component;
            match aggregate.affected.iter_mut().find(|a| &a.component == key) {
                Some(existing) => {
                    for range in &affected.ranges {
                        if !existing.ranges.contains(range) {
                            existing.ranges.push(range.clone());
                        }
                    }
                    for version in &affected.fixed {
                        if !existing.fixed.contains(version) {
                            existing.fixed.push(version.clone());
                        }
                    }
                }
                None => aggregate.affected.push(affected.clone()),
            }
        }
        for pair in explicit_identifiers(event) {
            if !aggregate.identifiers.contains(&pair) {
                aggregate.identifiers.push(pair);
            }
        }
        if let Some(url) = event.url.as_deref().and_then(canonical_url) {
            let pair = ("url".to_string(), url);
            if !aggregate.identifiers.contains(&pair) {
                aggregate.identifiers.push(pair);
            }
        }
        aggregate
            .source_entities
            .extend(event.entities.iter().map(str::to_string));
        aggregate.core_entities.extend(taxonomy.tag(event));
    }
    let source_entities = aggregate.source_entities.clone();
    aggregate
        .core_entities
        .retain(|e| !source_entities.contains(e));
    Some(aggregate)
}

/// Correlates validated events into stories; the event sink of every source.
pub struct Correlator<S> {
    store: S,
    taxonomy: Arc<Taxonomy>,
    clock: Arc<dyn Clock>,
    grace: SignedDuration,
}

impl<S: CorrelationStore> Correlator<S> {
    /// Creates the correlator with the instance's grace window.
    pub fn new(
        store: S,
        taxonomy: Arc<Taxonomy>,
        clock: Arc<dyn Clock>,
        grace: SignedDuration,
    ) -> Self {
        Self {
            store,
            taxonomy,
            clock,
            grace,
        }
    }

    /// Ingests events of one source.
    pub async fn ingest_events(
        &self,
        structured: bool,
        mirror: bool,
        events: Vec<NormalizedEvent>,
        backfill: bool,
    ) -> Result<IngestCounts, StoreError> {
        let mut total = IngestCounts::default();
        let mut events = events.into_iter().peekable();
        while events.peek().is_some() {
            let batch: Vec<NormalizedEvent> = events.by_ref().take(BATCH).collect();
            let context = Context {
                taxonomy: Arc::clone(&self.taxonomy),
                now: self.clock.now(),
                grace: self.grace,
            };
            let counts = self
                .store
                .correlate(Box::new(move |tx| {
                    let mut counts = IngestCounts::default();
                    for event in batch {
                        let meta = EventMeta {
                            backfill,
                            mirror,
                            title_priority: title_priority(structured, event.kind),
                            fetched_at: context.now,
                        };
                        context.ingest_one(tx, &event, &meta, &mut counts)?;
                    }
                    Ok(counts)
                }))
                .await?;
            total.new += counts.new;
            total.changed += counts.changed;
            total.unchanged += counts.unchanged;
        }
        Ok(total)
    }
}

impl<S: CorrelationStore> EventSink for Correlator<S> {
    async fn ingest(
        &self,
        source: &SourceInfo,
        events: Vec<NormalizedEvent>,
        backfill: bool,
    ) -> Result<usize, StoreError> {
        let counts = self
            .ingest_events(source.structured, source.mirror, events, backfill)
            .await?;
        Ok(counts.new + counts.changed)
    }
}

/// What the transaction closure needs.
struct Context {
    taxonomy: Arc<Taxonomy>,
    now: Timestamp,
    grace: SignedDuration,
}

impl Context {
    fn ingest_one(
        &self,
        tx: &mut dyn CorrelationTx,
        event: &NormalizedEvent,
        meta: &EventMeta,
        counts: &mut IngestCounts,
    ) -> Result<(), StoreError> {
        let hash = content_hash(event);
        match tx.find_event(&event.source, &event.source_item_id)? {
            Some(stored) if stored.content_hash == hash => {
                counts.unchanged += 1;
                Ok(())
            }
            Some(stored) => {
                tx.update_event(stored.id, event, &hash, meta)?;
                counts.changed += 1;
                self.recompute(tx, stored.story, meta.backfill, false)
            }
            None => {
                let (story, related) = Self::locate(tx, event)?;
                let (story, created) = match story {
                    Some(story) => (story, false),
                    None => (tx.insert_story(&self.placeholder(event, meta))?, true),
                };
                tx.insert_event(story, event, &hash, meta)?;
                for other in related {
                    tx.add_relation(story, other)?;
                }
                counts.new += 1;
                self.recompute(tx, story, meta.backfill, created)
            }
        }
    }

    /// Finds the story an event joins, merging stories its explicit identifiers reveal as the
    /// same. Returns the story (if any) and stories of the other family to link as related.
    fn locate(
        tx: &mut dyn CorrelationTx,
        event: &NormalizedEvent,
    ) -> Result<(Option<StoryId>, Vec<StoryId>), StoreError> {
        let family = event.kind.family();
        let mut related = Vec::new();
        let matches = |tx: &mut dyn CorrelationTx,
                       pairs: &[(String, String)],
                       related: &mut Vec<StoryId>|
         -> Result<Vec<StoryId>, StoreError> {
            let mut found = Vec::new();
            for (kind, value) in pairs {
                let Some(id) = tx.story_by_identifier(kind, value)? else {
                    continue;
                };
                let story = tx.story(id)?;
                if compatible(family, story.kind.family()) {
                    if !found.contains(&id) {
                        found.push(id);
                    }
                } else if !related.contains(&id) {
                    related.push(id);
                }
            }
            found.sort();
            Ok(found)
        };
        // 1. Explicit identifiers, including aliases: several matching stories are merged.
        let explicit = matches(tx, &explicit_identifiers(event), &mut related)?;
        if let Some((&into, others)) = explicit.split_first() {
            for &from in others {
                tx.merge_stories(from, into)?;
            }
            return Ok((Some(into), related));
        }
        // 2. Linked identifiers: joined only when they point to one story; text that mentions
        //    several stories never merges them and falls through to the canonical URL (docs/architecture/deduplication.md#conservative-merging).
        let linked = matches(tx, &linked_identifiers(event), &mut related)?;
        if linked.len() == 1 {
            return Ok((Some(linked[0]), related));
        }
        // 3. Canonical URL.
        if let Some(url) = event.url.as_deref().and_then(canonical_url) {
            let by_url = matches(tx, &[("url".to_string(), url)], &mut related)?;
            if let Some(&story) = by_url.first() {
                return Ok((Some(story), related));
            }
        }
        Ok((None, related))
    }

    fn placeholder(&self, event: &NormalizedEvent, meta: &EventMeta) -> StoryRecord {
        StoryRecord {
            kind: event.kind,
            title: event.title.clone(),
            title_priority: meta.title_priority,
            summary: None,
            url: None,
            published_at: None,
            severity: SeverityLevel::Unknown,
            cvss: None,
            epss: None,
            exploited: false,
            exploit_available: false,
            fix_available: false,
            first_seen_at: self.now,
            updated_at: self.now,
            eligible_at: None,
            needs_match: false,
            backfill: meta.backfill,
            compacted: false,
        }
    }

    /// Rebuilds a story from its events. A new or materially changed story gets a fresh grace
    /// window and is handed to the matcher, unless the change came from a backfill.
    fn recompute(
        &self,
        tx: &mut dyn CorrelationTx,
        story: StoryId,
        triggered_by_backfill: bool,
        created: bool,
    ) -> Result<(), StoreError> {
        let old = tx.story(story)?;
        let events = tx.story_events(story)?;
        let Some(agg) = aggregate(&events, &self.taxonomy) else {
            return Ok(());
        };
        let material = created
            || severity::increased(old.severity, agg.severity)
            || (agg.exploited && !old.exploited)
            || (agg.exploit_available && !old.exploit_available)
            || (agg.fix_available && !old.fix_available)
            || agg.kind != old.kind
            || old.compacted;
        let notify = material && !triggered_by_backfill;
        let record = StoryRecord {
            kind: agg.kind,
            title: agg.title,
            title_priority: agg.title_priority,
            summary: agg.summary,
            url: agg.url,
            published_at: agg.published_at,
            severity: agg.severity,
            cvss: agg.cvss,
            epss: agg.epss,
            exploited: agg.exploited,
            exploit_available: agg.exploit_available,
            fix_available: agg.fix_available,
            first_seen_at: old.first_seen_at,
            updated_at: self.now,
            eligible_at: if notify {
                self.now.checked_add(self.grace).ok()
            } else {
                old.eligible_at
            },
            needs_match: old.needs_match || notify,
            backfill: agg.backfill,
            compacted: false,
        };
        tx.update_story(story, &record)?;
        tx.add_identifiers(story, &agg.identifiers)?;
        tx.replace_affected(story, &agg.affected)?;
        tx.replace_entities(story, &agg.source_entities, &agg.core_entities)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_urls_drop_tracking_fragments_and_trailing_slashes() {
        // deduplication.md: lowercase host, no tracking parameters, no fragment, no trailing slash.
        let cases = [
            (
                "https://Example.COM/Advisory/?utm_source=x&id=7#top",
                "https://example.com/Advisory?id=7",
            ),
            ("https://example.com/a/", "https://example.com/a"),
            ("https://example.com/", "https://example.com"),
            (
                "http://example.com/x?fbclid=1&gclid=2",
                "http://example.com/x",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(canonical_url(input).as_deref(), Some(expected), "{input}");
        }
        assert_eq!(canonical_url("javascript:alert(1)"), None);
    }

    #[test]
    fn title_priority_prefers_databases_then_feeds_then_news() {
        assert!(
            title_priority(true, EventKind::Vulnerability)
                > title_priority(false, EventKind::Release)
        );
        assert!(title_priority(false, EventKind::Release) > title_priority(true, EventKind::News));
    }
}
