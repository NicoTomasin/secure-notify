// SPDX-License-Identifier: AGPL-3.0-or-later

//! Matching: relevance, mutes, rules, and the minimum severity
//! (docs/architecture/matching-and-rules.md). For each story that is new or materially changed
//! and past its grace window, candidates come from inverted indexes (component → inventory
//! entries, entity → watches), never from a scan of every subscriber.

use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::sync::Arc;

use jiff::Timestamp;
use secure_notify_model::{Affected, EventKind, SeverityLevel};
use secure_notify_versions::{Evaluation, evaluate, fixes_for};
use serde::{Deserialize, Serialize};

use crate::correlation::StoryId;
use crate::ports::{Clock, StoreError};
use crate::severity;
use crate::subscribers::SubscriberId;

/// Relevance levels, highest first (docs/architecture/matching-and-rules.md#relevance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Relevance {
    /// A verified watched identity appears in a breach (not computed in the MVP).
    Identity,
    /// A component's exact version is inside an affected range.
    ExactVersion,
    /// An affected component is in the inventory, its version unknown.
    Component,
    /// A watched product or vendor (not computed in the MVP).
    Product,
    /// A watched organization (not computed in the MVP).
    Organization,
    /// A watched technology.
    Technology,
}

impl Relevance {
    /// The level's documented name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::ExactVersion => "exact-version",
            Self::Component => "component",
            Self::Product => "product",
            Self::Organization => "organization",
            Self::Technology => "technology",
        }
    }
}

/// An inventory entry that made a story relevant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MatchedEntry {
    /// Component purl.
    pub component: String,
    /// Version in the inventory, if known.
    pub version: Option<String>,
    /// Fixed versions of the advisory for that component.
    pub fixed: Vec<String>,
}

/// Why a story reaches a subscriber; stored with the queue item for the message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    /// Highest relevance.
    pub relevance: Relevance,
    /// Inventory entries behind `exact-version` or `component` relevance.
    pub entries: Vec<MatchedEntry>,
    /// Watched entities behind `technology` relevance.
    pub entities: Vec<String>,
}

/// The part of a story that update notices compare (docs/architecture/deduplication.md#update-notices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Severity level.
    pub severity: SeverityLevel,
    /// Known to be exploited.
    pub exploited: bool,
    /// Public exploit available.
    pub exploit_available: bool,
    /// Fix available.
    pub fix_available: bool,
}

/// Whether a story changed materially since a snapshot was sent. The MVP's triggers are fixed:
/// severity increase, `exploited`, `exploit_available`, fix available.
pub fn update_due(sent: &Snapshot, now: &Snapshot) -> bool {
    severity::increased(sent.severity, now.severity)
        || (now.exploited && !sent.exploited)
        || (now.exploit_available && !sent.exploit_available)
        || (now.fix_available && !sent.fix_available)
}

/// Match conditions of a rule; absent fields match anything.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleMatch {
    /// Relevance levels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relevance: Vec<Relevance>,
    /// Event kinds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kind: Vec<EventKind>,
    /// A taxonomy ID; a trailing `*` matches a prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    /// A purl or CPE prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// Source IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source: Vec<String>,
    /// `exploited` flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploited: Option<bool>,
    /// `exploit_available` flag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exploit_available: Option<bool>,
    /// Minimum EPSS score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_epss: Option<f64>,
}

/// What a matching rule does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    /// Notify, subject to the rule's `min_severity`.
    Notify,
    /// Do not notify.
    Ignore,
}

/// A rule (docs/architecture/matching-and-rules.md#rules).
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Conditions.
    pub conditions: RuleMatch,
    /// Action.
    pub action: RuleAction,
    /// Severity threshold of a `notify` rule.
    pub min_severity: Option<SeverityLevel>,
}

/// The default rules for new subscribers
/// (docs/architecture/matching-and-rules.md#default-rules-for-new-subscribers), row by row.
pub fn default_rules() -> Vec<Rule> {
    let notify = |conditions: RuleMatch, min: Option<SeverityLevel>| Rule {
        conditions,
        action: RuleAction::Notify,
        min_severity: min,
    };
    let relevance = |level: Relevance, kind: Vec<EventKind>| RuleMatch {
        relevance: vec![level],
        kind,
        ..RuleMatch::default()
    };
    vec![
        notify(
            RuleMatch {
                exploited: Some(true),
                ..RuleMatch::default()
            },
            None,
        ),
        notify(relevance(Relevance::Identity, vec![]), None),
        notify(
            relevance(Relevance::ExactVersion, vec![]),
            Some(SeverityLevel::Medium),
        ),
        notify(
            relevance(Relevance::Component, vec![]),
            Some(SeverityLevel::High),
        ),
        notify(
            relevance(Relevance::Product, vec![]),
            Some(SeverityLevel::High),
        ),
        notify(
            relevance(
                Relevance::Organization,
                vec![EventKind::Breach, EventKind::Attack],
            ),
            Some(SeverityLevel::Medium),
        ),
        notify(
            relevance(Relevance::Organization, vec![EventKind::Outage]),
            Some(SeverityLevel::High),
        ),
        notify(
            relevance(Relevance::Technology, vec![]),
            Some(SeverityLevel::High),
        ),
    ]
}

/// What rules see of a story.
#[derive(Debug, Clone, PartialEq)]
pub struct StoryFacts {
    /// Kind.
    pub kind: EventKind,
    /// Severity level.
    pub severity: SeverityLevel,
    /// Known to be exploited.
    pub exploited: bool,
    /// Public exploit available.
    pub exploit_available: bool,
    /// Fix available.
    pub fix_available: bool,
    /// EPSS score.
    pub epss: Option<f64>,
    /// Sources reporting it.
    pub sources: Vec<String>,
    /// Entities it is tagged with.
    pub entities: Vec<String>,
    /// Affected components with their ranges.
    pub affected: Vec<Affected>,
    /// When the story can be dispatched.
    pub eligible_at: Timestamp,
}

impl StoryFacts {
    /// The snapshot compared by update notices.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            severity: self.severity,
            exploited: self.exploited,
            exploit_available: self.exploit_available,
            fix_available: self.fix_available,
        }
    }
}

fn rule_matches(rule: &RuleMatch, story: &StoryFacts, reason: &Reason) -> bool {
    let entity_matches = |pattern: &str| match pattern.strip_suffix('*') {
        Some(prefix) => story.entities.iter().any(|e| e.starts_with(prefix)),
        None => story.entities.iter().any(|e| e == pattern),
    };
    (rule.relevance.is_empty() || rule.relevance.contains(&reason.relevance))
        && (rule.kind.is_empty() || rule.kind.contains(&story.kind))
        && rule.entity.as_deref().is_none_or(entity_matches)
        && rule.component.as_deref().is_none_or(|prefix| {
            story
                .affected
                .iter()
                .filter_map(|a| a.component.purl.as_deref().or(a.component.cpe.as_deref()))
                .any(|c| c.starts_with(prefix))
        })
        && (rule.source.is_empty() || rule.source.iter().any(|s| story.sources.contains(s)))
        && rule.exploited.is_none_or(|v| v == story.exploited)
        && rule
            .exploit_available
            .is_none_or(|v| v == story.exploit_available)
        && rule
            .min_epss
            .is_none_or(|min| story.epss.is_some_and(|e| e >= min))
}

/// Rules, then the minimum severity: whether a relevant story is notified.
pub fn decide(
    rules: &[Rule],
    min_severity: Option<SeverityLevel>,
    story: &StoryFacts,
    reason: &Reason,
) -> bool {
    let Some(rule) = rules
        .iter()
        .find(|r| rule_matches(&r.conditions, story, reason))
    else {
        return false;
    };
    let notified = match rule.action {
        RuleAction::Ignore => false,
        RuleAction::Notify => rule
            .min_severity
            .is_none_or(|min| severity::passes(story.severity, min)),
    };
    // The subscriber's minimum severity applies after the rules, except to exploited stories.
    notified
        && (story.exploited || min_severity.is_none_or(|min| severity::passes(story.severity, min)))
}

/// A mute (docs/architecture/delivery.md#inline-actions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mute {
    /// One story.
    Story(StoryId),
    /// A taxonomy entity.
    Entity(String),
    /// A component, any version.
    Component(String),
    /// One version of a component (`purl@version`).
    ComponentVersion(String, String),
}

/// One candidate subscriber of a story, with what matched them and their settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The subscriber.
    pub subscriber: SubscriberId,
    /// Inventory entries of affected components.
    pub entries: Vec<(String, Option<String>)>,
    /// Watched entities among the story's entities.
    pub watched: Vec<String>,
    /// Custom rules; empty means the defaults.
    pub rules: Vec<Rule>,
    /// Minimum severity.
    pub min_severity: Option<SeverityLevel>,
    /// Active mutes.
    pub mutes: Vec<Mute>,
    /// The last snapshot delivered, if the story was delivered.
    pub delivered: Option<Snapshot>,
}

/// The relevance of a story for a candidate, after mutes; `None` when not relevant.
pub fn relevance(story_id: StoryId, story: &StoryFacts, candidate: &Candidate) -> Option<Reason> {
    let muted = |mute: &Mute| match mute {
        Mute::Story(id) => *id == story_id,
        Mute::Entity(entity) => story.entities.contains(entity),
        _ => false,
    };
    if candidate.mutes.iter().any(muted) {
        return None;
    }
    let entry_muted = |component: &str, version: Option<&str>| {
        candidate.mutes.iter().any(|m| match m {
            Mute::Component(c) => c == component,
            Mute::ComponentVersion(c, v) => c == component && Some(v.as_str()) == version,
            _ => false,
        })
    };
    let mut exact = Vec::new();
    let mut unknown = Vec::new();
    for affected in &story.affected {
        let Some(component) = affected
            .component
            .purl
            .as_deref()
            .or(affected.component.cpe.as_deref())
        else {
            continue;
        };
        let entries: Vec<&Option<String>> = candidate
            .entries
            .iter()
            .filter(|(c, v)| c == component && !entry_muted(c, v.as_deref()))
            .map(|(_, v)| v)
            .collect();
        for version in entries {
            let entry = MatchedEntry {
                component: component.to_string(),
                version: version.clone(),
                fixed: match version {
                    Some(v) => fixes_for(affected, v),
                    None => affected.fixed.clone(),
                },
            };
            match version.as_deref().map(|v| evaluate(affected, v)) {
                Some(Evaluation::Affected) => exact.push(entry),
                // A known version outside every range is not relevant for that entry.
                Some(Evaluation::NotAffected) => {}
                Some(Evaluation::Unknown) | None => unknown.push(entry),
            }
        }
    }
    let watched: Vec<String> = candidate
        .watched
        .iter()
        .filter(|e| e.starts_with("tech:") && story.entities.contains(e))
        .cloned()
        .collect();
    let (relevance, entries) = if !exact.is_empty() {
        (Relevance::ExactVersion, exact)
    } else if !unknown.is_empty() {
        (Relevance::Component, unknown)
    } else if !watched.is_empty() {
        (Relevance::Technology, Vec::new())
    } else {
        return None;
    };
    Some(Reason {
        relevance,
        entries,
        entities: watched,
    })
}

/// Kind of a queue item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    /// First notification of the story.
    New,
    /// Update notice for a delivered story.
    Update,
}

impl NoticeKind {
    /// The stored name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Update => "update",
        }
    }
}

/// A notification to queue.
#[derive(Debug, Clone, PartialEq)]
pub struct Enqueue {
    /// Recipient.
    pub subscriber: SubscriberId,
    /// New story or update.
    pub kind: NoticeKind,
    /// Why it is relevant.
    pub reason: Reason,
    /// When it is due: `instant` mode, after the grace window.
    pub due_at: Timestamp,
}

/// Decides, for every candidate, whether to queue the story and as what.
pub fn plan(
    story_id: StoryId,
    story: &StoryFacts,
    candidates: &[Candidate],
    now: Timestamp,
) -> Vec<Enqueue> {
    let defaults = default_rules();
    let snapshot = story.snapshot();
    let mut queue = Vec::new();
    for candidate in candidates {
        let Some(reason) = relevance(story_id, story, candidate) else {
            continue;
        };
        let rules = if candidate.rules.is_empty() {
            &defaults
        } else {
            &candidate.rules
        };
        if !decide(rules, candidate.min_severity, story, &reason) {
            continue;
        }
        let kind = match &candidate.delivered {
            None => NoticeKind::New,
            Some(sent) if update_due(sent, &snapshot) => NoticeKind::Update,
            Some(_) => continue,
        };
        queue.push(Enqueue {
            subscriber: candidate.subscriber,
            kind,
            reason,
            due_at: story.eligible_at.max(now),
        });
    }
    queue
}

/// Persistence for matching.
pub trait MatchStore: Send + Sync {
    /// Stories waiting for the matcher whose grace window ended at `now`, oldest first.
    fn stories_to_match(
        &self,
        now: Timestamp,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<StoryId>, StoreError>> + Send;

    /// A story's facts and its candidates, found through the inverted indexes.
    fn match_input(
        &self,
        story: StoryId,
        now: Timestamp,
    ) -> impl Future<Output = Result<Option<(StoryFacts, Vec<Candidate>)>, StoreError>> + Send;

    /// Queues the notifications (replacing pending items of the same kind) and marks the story
    /// as matched, atomically.
    fn enqueue(
        &self,
        story: StoryId,
        items: Vec<Enqueue>,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// The matcher.
pub struct Matcher<S> {
    store: S,
    clock: Arc<dyn Clock>,
}

impl<S: MatchStore> Matcher<S> {
    /// Creates the matcher.
    pub fn new(store: S, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// Matches every due story. Returns how many stories were processed.
    pub async fn run_once(&self) -> Result<usize, StoreError> {
        let now = self.clock.now();
        let mut processed = 0;
        loop {
            let stories = self.store.stories_to_match(now, 100).await?;
            if stories.is_empty() {
                return Ok(processed);
            }
            for story in stories {
                let items = match self.store.match_input(story, now).await? {
                    Some((facts, candidates)) => plan(story, &facts, &candidates, now),
                    None => Vec::new(),
                };
                self.store.enqueue(story, items, now).await?;
                processed += 1;
            }
        }
    }
}

/// Distinct values, for building candidate lists.
pub fn distinct<T: Ord + Clone>(items: &[T]) -> Vec<T> {
    items
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Groups `(key, value)` pairs.
pub fn group<K: Ord, V>(pairs: impl IntoIterator<Item = (K, V)>) -> BTreeMap<K, Vec<V>> {
    let mut map: BTreeMap<K, Vec<V>> = BTreeMap::new();
    for (k, v) in pairs {
        map.entry(k).or_default().push(v);
    }
    map
}
