// SPDX-License-Identifier: AGPL-3.0-or-later

//! The dispatcher with fakes at its ports (docs/architecture/delivery.md#queue-and-dispatcher,
//! docs/architecture/deduplication.md#double-check-before-dispatch).

#![allow(clippy::unwrap_used, clippy::unused_async_trait_impl)]

use std::sync::{Arc, Mutex};

use jiff::Timestamp;
use secure_notify_core::correlation::StoryId;
use secure_notify_core::delivery::{
    DeliveryStore, Dispatcher, Notice, Outbox, Prepared, QueueItem, StoryView,
};
use secure_notify_core::matching::{NoticeKind, Reason, Relevance, Snapshot};
use secure_notify_core::ports::{Clock, SendError, StoreError};
use secure_notify_core::sources::BoxFuture;
use secure_notify_core::subscribers::{EndpointRef, SubscriberId};
use secure_notify_model::{EventKind, SeverityLevel};

struct Fixed;

impl Clock for Fixed {
    fn now(&self) -> Timestamp {
        "2026-11-27T12:00:00Z".parse().unwrap()
    }
}

#[derive(Default)]
struct Log {
    events: Mutex<Vec<String>>,
}

impl Log {
    fn push(&self, event: String) {
        self.events.lock().unwrap().push(event);
    }
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
}

/// One queue item whose story and ledger are set per test.
struct Store {
    item: QueueItem,
    story: StoryView,
    delivered: Option<Snapshot>,
    log: Arc<Log>,
}

impl DeliveryStore for Store {
    async fn due_items(&self, _: Timestamp, _: usize) -> Result<Vec<QueueItem>, StoreError> {
        Ok(vec![self.item.clone()])
    }
    async fn prepare(&self, _: &QueueItem) -> Result<Option<Prepared>, StoreError> {
        Ok(Some(Prepared {
            story: self.story.clone(),
            delivered: self.delivered,
            endpoint: Some((
                10,
                EndpointRef {
                    platform: "test".to_string(),
                    address: "1".to_string(),
                },
            )),
            language: "en".to_string(),
        }))
    }
    async fn complete(
        &self,
        _: &QueueItem,
        snapshot: Snapshot,
        _: i64,
        _: Timestamp,
    ) -> Result<bool, StoreError> {
        self.log
            .push(format!("ledger exploited={}", snapshot.exploited));
        Ok(true)
    }
    async fn retry_later(
        &self,
        _: &QueueItem,
        _: i64,
        error: &str,
        retry_at: Timestamp,
        _: Timestamp,
    ) -> Result<(), StoreError> {
        self.log.push(format!("retry at {retry_at} after {error}"));
        Ok(())
    }
    async fn drop_item(&self, _: &QueueItem) -> Result<(), StoreError> {
        self.log.push("dropped".to_string());
        Ok(())
    }
    async fn endpoint_gone(&self, endpoint: i64) -> Result<(), StoreError> {
        self.log.push(format!("endpoint {endpoint} gone"));
        Ok(())
    }
}

struct FakeOutbox {
    result: Result<(), SendError>,
    log: Arc<Log>,
}

impl Outbox for FakeOutbox {
    fn deliver<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        notice: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            self.log.push(format!("sent {:?}", notice.kind));
            self.result.clone()
        })
    }

    fn deliver_report<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a secure_notify_core::scan::ScanReport,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move { self.result.clone() })
    }

    fn deliver_alert<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a secure_notify_core::health::SourceAlert,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move { self.result.clone() })
    }
}

fn story(exploited: bool) -> StoryView {
    StoryView {
        id: StoryId(1),
        kind: EventKind::Vulnerability,
        title: "Command Injection in lodash".to_string(),
        url: None,
        severity: SeverityLevel::High,
        exploited,
        exploit_available: false,
        fix_available: true,
        cvss: None,
        epss: None,
        summary: None,
        entities: Vec::new(),
        sources: 1,
    }
}

fn item(kind: NoticeKind) -> QueueItem {
    QueueItem {
        id: 5,
        subscriber: SubscriberId(1),
        story: StoryId(1),
        kind,
        reason: Reason {
            relevance: Relevance::ExactVersion,
            entries: Vec::new(),
            entities: Vec::new(),
        },
        attempts: 2,
    }
}

async fn run(
    kind: NoticeKind,
    exploited_now: bool,
    delivered: Option<Snapshot>,
    result: Result<(), SendError>,
) -> Vec<String> {
    let log = Arc::new(Log::default());
    let store = Store {
        item: item(kind),
        story: story(exploited_now),
        delivered,
        log: log.clone(),
    };
    let outbox = Arc::new(FakeOutbox {
        result,
        log: log.clone(),
    });
    Dispatcher::new(store, outbox, Arc::new(Fixed))
        .run_once()
        .await
        .unwrap();
    log.events()
}

#[tokio::test]
async fn a_story_already_in_the_ledger_is_not_sent_again() {
    let sent = Some(story(false).snapshot());
    assert_eq!(run(NoticeKind::New, false, sent, Ok(())).await, ["dropped"]);
}

#[tokio::test]
async fn a_new_story_is_sent_and_recorded() {
    assert_eq!(
        run(NoticeKind::New, false, None, Ok(())).await,
        ["sent New", "ledger exploited=false"]
    );
}

#[tokio::test]
async fn an_update_is_sent_only_while_the_change_still_stands() {
    let sent = Some(story(false).snapshot());
    assert_eq!(
        run(NoticeKind::Update, true, sent, Ok(())).await,
        ["sent Update", "ledger exploited=true"]
    );
    assert_eq!(
        run(NoticeKind::Update, false, sent, Ok(())).await,
        ["dropped"]
    );
}

#[tokio::test]
async fn temporary_failures_are_retried_with_backoff_without_a_ledger_row() {
    // Third attempt: four minutes later.
    let events = run(
        NoticeKind::New,
        false,
        None,
        Err(SendError::Temporary("timeout".to_string())),
    )
    .await;
    assert_eq!(
        events,
        ["sent New", "retry at 2026-11-27T12:04:00Z after timeout"]
    );
}

#[tokio::test]
async fn a_lost_endpoint_is_deactivated() {
    let events = run(NoticeKind::New, false, None, Err(SendError::EndpointGone)).await;
    assert_eq!(events, ["sent New", "endpoint 10 gone"]);
}
