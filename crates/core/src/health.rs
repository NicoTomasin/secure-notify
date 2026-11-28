// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source health for `/status` and failure-streak alerts to admins
//! (docs/operations/self-hosting.md#operations).

use std::future::Future;
use std::sync::Arc;

use jiff::Timestamp;

use crate::delivery::Outbox;
use crate::ports::{SendError, StoreError};
use crate::scan::Mirror;
use crate::sources::{
    BoxFuture, FAILURES, HealthObserver, ITEMS, LAST_ERROR, LAST_FAILURE, LAST_SUCCESS, SourceInfo,
    SourceState, SourceStateStore,
};
use crate::subscribers::EndpointRef;

/// A source known to the instance, enabled or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    /// Source ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether it runs.
    pub enabled: bool,
    /// Whether it mirrors a whole database.
    pub mirror: bool,
    /// Purl types a mirror covers.
    pub ecosystems: Vec<String>,
}

impl SourceEntry {
    /// The entry of a source.
    pub fn new(info: &SourceInfo, enabled: bool) -> Self {
        Self {
            id: info.id.clone(),
            name: info.name.clone(),
            enabled,
            mirror: info.mirror,
            ecosystems: info.ecosystems.clone(),
        }
    }
}

/// The enabled mirrors among `sources`.
pub fn mirrors(sources: &[SourceEntry]) -> Arc<[Mirror]> {
    sources
        .iter()
        .filter(|s| s.enabled && s.mirror)
        .map(|s| Mirror {
            source: s.id.clone(),
            ecosystems: s.ecosystems.clone(),
        })
        .collect()
}

/// The health of one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceHealth {
    /// Turned off by the operator.
    Disabled,
    /// The first synchronization has not completed yet.
    FirstSync,
    /// The last run succeeded.
    Healthy {
        /// Last successful run.
        last_success: Timestamp,
    },
    /// The last runs failed.
    Failing {
        /// Consecutive failed runs.
        failures: u32,
        /// Last failed run.
        last_failure: Option<Timestamp>,
        /// Its error.
        error: String,
        /// Last successful run, if any.
        last_success: Option<Timestamp>,
    },
}

/// One line of `/status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceStatus {
    /// Display name.
    pub name: String,
    /// Its health.
    pub health: SourceHealth,
    /// Events accepted since the source was first run.
    pub items: u64,
}

/// The health of a source from its stored state.
pub fn source_status(entry: &SourceEntry, state: &SourceState) -> SourceStatus {
    let time = |key: &str| state.get(key).and_then(|t| t.parse::<Timestamp>().ok());
    let failures: u32 = state
        .get(FAILURES)
        .and_then(|f| f.parse().ok())
        .unwrap_or(0);
    let health = if !entry.enabled {
        SourceHealth::Disabled
    } else if failures > 0 {
        SourceHealth::Failing {
            failures,
            last_failure: time(LAST_FAILURE),
            error: state.get(LAST_ERROR).cloned().unwrap_or_default(),
            last_success: time(LAST_SUCCESS),
        }
    } else {
        // A successful run completes the first synchronization and records its time.
        match time(LAST_SUCCESS) {
            Some(last_success) => SourceHealth::Healthy { last_success },
            None => SourceHealth::FirstSync,
        }
    };
    SourceStatus {
        name: entry.name.clone(),
        health,
        items: state.get(ITEMS).and_then(|n| n.parse().ok()).unwrap_or(0),
    }
}

/// Instance counters for `/status`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstanceStats {
    /// Active subscribers.
    pub subscribers: u64,
    /// Stories, compacted ones included.
    pub stories: u64,
    /// Stored events.
    pub events: u64,
    /// Notifications waiting in the queue.
    pub queued: u64,
    /// Upload entries waiting for their initial scan.
    pub pending_scans: u64,
}

/// Persistence for status and alerts.
pub trait HealthStore: Send + Sync {
    /// Instance counters.
    fn instance_stats(&self) -> impl Future<Output = Result<InstanceStats, StoreError>> + Send;

    /// Where admins are reached: the endpoint notifications go to and the language, per active
    /// admin with an active endpoint.
    fn admin_recipients(
        &self,
    ) -> impl Future<Output = Result<Vec<(EndpointRef, String)>, StoreError>> + Send;
}

/// `/status`.
pub struct Status<S> {
    store: S,
    sources: Arc<[SourceEntry]>,
}

impl<S: HealthStore + SourceStateStore> Status<S> {
    /// Creates the service over the instance's sources.
    pub fn new(store: S, sources: Arc<[SourceEntry]>) -> Self {
        Self { store, sources }
    }

    /// Each source's health, in registry order, and the instance counters.
    pub async fn report(&self) -> Result<(Vec<SourceStatus>, InstanceStats), StoreError> {
        let mut statuses = Vec::with_capacity(self.sources.len());
        for entry in self.sources.iter() {
            let state = self.store.source_state(&entry.id).await?;
            statuses.push(source_status(entry, &state));
        }
        Ok((statuses, self.store.instance_stats().await?))
    }
}

/// A failure streak, as told to admins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAlert {
    /// Display name of the source.
    pub source: String,
    /// Consecutive failed runs.
    pub failures: u32,
    /// The last error.
    pub error: String,
}

/// Tells every admin about failure streaks.
pub struct Alerter<S> {
    store: S,
    outbox: Arc<dyn Outbox>,
}

impl<S: HealthStore> Alerter<S> {
    /// Creates the alerter.
    pub fn new(store: S, outbox: Arc<dyn Outbox>) -> Self {
        Self { store, outbox }
    }

    /// Sends `alert` to every admin; returns how many were reached.
    pub async fn alert(&self, alert: &SourceAlert) -> Result<usize, StoreError> {
        let mut reached = 0;
        for (endpoint, language) in self.store.admin_recipients().await? {
            match self.outbox.deliver_alert(&endpoint, &language, alert).await {
                Ok(()) => reached += 1,
                Err(SendError::EndpointGone | SendError::Temporary(_)) => {
                    tracing::warn!(source = %alert.source, "source alert not delivered to an admin");
                }
            }
        }
        Ok(reached)
    }
}

impl<S: HealthStore> HealthObserver for Alerter<S> {
    fn source_failing<'a>(
        &'a self,
        source: &'a SourceInfo,
        failures: u32,
        error: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let alert = SourceAlert {
                source: source.name.clone(),
                failures,
                error: error.to_string(),
            };
            if let Err(error) = self.alert(&alert).await {
                tracing::error!(source = %source.id, error = %error, "cannot alert admins");
            }
        })
    }
}
