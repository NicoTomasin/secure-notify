// SPDX-License-Identifier: AGPL-3.0-or-later

//! The dispatcher (docs/architecture/delivery.md#queue-and-dispatcher): takes due queue items,
//! runs the double check against the ledger, sends through the subscriber's primary endpoint, and
//! writes the ledger in the same transaction that removes the queue item
//! (docs/architecture/deduplication.md#double-check-before-dispatch).

use std::future::Future;
use std::sync::Arc;

use jiff::{SignedDuration, Timestamp};
use secure_notify_model::{EventKind, SeverityLevel};

use crate::correlation::StoryId;
use crate::matching::{NoticeKind, Reason, Snapshot, update_due};
use crate::ports::{Clock, SendError, StoreError};
use crate::sources::BoxFuture;
use crate::subscribers::{EndpointRef, SubscriberId};

/// Longest wait between attempts of a failing delivery.
pub const MAX_RETRY_DELAY: SignedDuration = SignedDuration::from_hours(1);

/// A queued notification.
#[derive(Debug, Clone, PartialEq)]
pub struct QueueItem {
    /// Row ID.
    pub id: i64,
    /// Recipient.
    pub subscriber: SubscriberId,
    /// Story.
    pub story: StoryId,
    /// New or update.
    pub kind: NoticeKind,
    /// Why it is relevant.
    pub reason: Reason,
    /// Failed attempts so far.
    pub attempts: u32,
}

/// What a message shows about a story (docs/architecture/delivery.md#message-content).
#[derive(Debug, Clone, PartialEq)]
pub struct StoryView {
    /// Story ID, for inline actions.
    pub id: StoryId,
    /// Kind.
    pub kind: EventKind,
    /// Title, as published.
    pub title: String,
    /// Canonical link.
    pub url: Option<String>,
    /// Severity level.
    pub severity: SeverityLevel,
    /// Known to be exploited.
    pub exploited: bool,
    /// Public exploit available.
    pub exploit_available: bool,
    /// Fix available.
    pub fix_available: bool,
    /// CVSS version and score.
    pub cvss: Option<(String, f64)>,
    /// EPSS score.
    pub epss: Option<f64>,
    /// Summary, as published.
    pub summary: Option<String>,
    /// Entities the story is tagged with.
    pub entities: Vec<String>,
    /// Number of sources reporting it.
    pub sources: usize,
}

impl StoryView {
    /// The snapshot recorded in the ledger.
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            severity: self.severity,
            exploited: self.exploited,
            exploit_available: self.exploit_available,
            fix_available: self.fix_available,
        }
    }
}

/// A notification ready to render.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// New story or update notice.
    pub kind: NoticeKind,
    /// The story.
    pub story: StoryView,
    /// Why it reaches the subscriber.
    pub reason: Reason,
    /// For updates, the snapshot sent before.
    pub previous: Option<Snapshot>,
}

/// Everything the dispatcher needs to deliver one item.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The story, as it is now.
    pub story: StoryView,
    /// The last snapshot delivered to the subscriber, if any.
    pub delivered: Option<Snapshot>,
    /// The primary active endpoint and its row ID.
    pub endpoint: Option<(i64, EndpointRef)>,
    /// The subscriber's language.
    pub language: String,
}

/// Renders and sends a notice to an endpoint; implemented by the binary over the conversation
/// layer and the platform plugins.
pub trait Outbox: Send + Sync {
    /// Delivers `notice` to `endpoint` in `language`.
    fn deliver<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        notice: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>>;
}

/// Persistence for delivery.
pub trait DeliveryStore: Send + Sync {
    /// Queue items due at `now`, oldest first.
    fn due_items(
        &self,
        now: Timestamp,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<QueueItem>, StoreError>> + Send;

    /// The current story, the ledger, and the endpoint of an item; `None` if the story is gone.
    fn prepare(
        &self,
        item: &QueueItem,
    ) -> impl Future<Output = Result<Option<Prepared>, StoreError>> + Send;

    /// Records the delivery: in one transaction, re-checks the ledger, writes the ledger row and
    /// the attempt, and removes the queue item. Returns `false` if the ledger already held it.
    fn complete(
        &self,
        item: &QueueItem,
        snapshot: Snapshot,
        endpoint: i64,
        now: Timestamp,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Records a failed attempt and when to retry.
    fn retry_later(
        &self,
        item: &QueueItem,
        endpoint: i64,
        error: &str,
        retry_at: Timestamp,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Removes a queue item that must not be sent.
    fn drop_item(&self, item: &QueueItem) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Deactivates an endpoint the bot lost; a subscriber left without active endpoints is
    /// deactivated and its queue dropped (docs/architecture/subscribers.md#subscribers-and-endpoints).
    fn endpoint_gone(&self, endpoint: i64) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// What a dispatcher pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DispatchCounts {
    /// Messages sent.
    pub sent: usize,
    /// Items dropped by the double check or for lack of an endpoint.
    pub dropped: usize,
    /// Items that will be retried.
    pub retried: usize,
}

/// The delay before retry number `attempts` (1-based): one minute, doubled each time, up to an
/// hour.
pub fn retry_delay(attempts: u32) -> SignedDuration {
    let minutes = 1_i64 << attempts.saturating_sub(1).min(10);
    SignedDuration::from_mins(minutes).min(MAX_RETRY_DELAY)
}

/// The dispatcher.
pub struct Dispatcher<S> {
    store: S,
    outbox: Arc<dyn Outbox>,
    clock: Arc<dyn Clock>,
}

impl<S: DeliveryStore> Dispatcher<S> {
    /// Creates the dispatcher.
    pub fn new(store: S, outbox: Arc<dyn Outbox>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            outbox,
            clock,
        }
    }

    /// Sends every due item once.
    pub async fn run_once(&self) -> Result<DispatchCounts, StoreError> {
        let now = self.clock.now();
        let mut counts = DispatchCounts::default();
        for item in self.store.due_items(now, 500).await? {
            let Some(prepared) = self.store.prepare(&item).await? else {
                self.store.drop_item(&item).await?;
                counts.dropped += 1;
                continue;
            };
            let current = prepared.story.snapshot();
            // Double check at dispatch: a story is sent once, then only material updates.
            let still_due = match (item.kind, &prepared.delivered) {
                (NoticeKind::New, None) => true,
                (NoticeKind::Update, Some(sent)) => update_due(sent, &current),
                (NoticeKind::New, Some(_)) | (NoticeKind::Update, None) => false,
            };
            let Some((endpoint_id, endpoint)) = prepared.endpoint.clone() else {
                self.store.drop_item(&item).await?;
                counts.dropped += 1;
                continue;
            };
            if !still_due {
                self.store.drop_item(&item).await?;
                counts.dropped += 1;
                continue;
            }
            let notice = Notice {
                kind: item.kind,
                story: prepared.story.clone(),
                reason: item.reason.clone(),
                previous: prepared.delivered,
            };
            match self
                .outbox
                .deliver(&endpoint, &prepared.language, &notice)
                .await
            {
                Ok(()) => {
                    if self
                        .store
                        .complete(&item, current, endpoint_id, now)
                        .await?
                    {
                        counts.sent += 1;
                    } else {
                        counts.dropped += 1;
                    }
                }
                Err(SendError::EndpointGone) => {
                    self.store.endpoint_gone(endpoint_id).await?;
                    counts.dropped += 1;
                }
                Err(SendError::Temporary(error)) => {
                    let retry_at = now
                        .checked_add(retry_delay(item.attempts + 1))
                        .unwrap_or(now);
                    self.store
                        .retry_later(&item, endpoint_id, &error, retry_at, now)
                        .await?;
                    counts.retried += 1;
                }
            }
        }
        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_back_off_from_a_minute_to_an_hour() {
        let minutes: Vec<i64> = (1..=8).map(|a| retry_delay(a).as_mins()).collect();
        assert_eq!(minutes, [1, 2, 4, 8, 16, 32, 60, 60]);
    }
}

/// What the "Details" inline action shows (docs/architecture/delivery.md#inline-actions).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDetails {
    /// Title.
    pub title: String,
    /// Identifiers as `kind:value`, URLs excluded.
    pub identifiers: Vec<String>,
    /// Sources reporting the story.
    pub sources: Vec<String>,
    /// Affected components with their fixed versions.
    pub affected: Vec<(String, Vec<String>)>,
    /// Related stories of the other family, by title.
    pub related: Vec<String>,
}

/// Persistence for story details.
pub trait DetailsStore: Send + Sync {
    /// The details of a story, resolving merged stories; `None` if it no longer exists.
    fn story_details(
        &self,
        story: StoryId,
    ) -> impl Future<Output = Result<Option<StoryDetails>, StoreError>> + Send;
}
