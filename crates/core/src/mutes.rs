// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mutes: story, entity, component, or component version, optionally time-limited
//! (docs/architecture/delivery.md#inline-actions, docs/architecture/matching-and-rules.md#rules).

use std::future::Future;
use std::sync::Arc;

use jiff::Timestamp;

use crate::ports::{Clock, StoreError};
use crate::subscribers::SubscriberId;
use crate::time::parse_duration;

/// Scopes of a mute, as stored.
pub const SCOPES: &[&str] = &["story", "entity", "component", "component-version"];

/// A stored mute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuteEntry {
    /// Row ID, shown by `/mutes`.
    pub id: i64,
    /// `story`, `entity`, `component`, or `component-version`.
    pub scope: String,
    /// Story ID, entity ID, purl, or `purl@version`.
    pub value: String,
    /// End of a time-limited mute.
    pub until: Option<Timestamp>,
}

/// Persistence for mutes.
pub trait MuteStore: Send + Sync {
    /// Adds or replaces a mute.
    fn add_mute(
        &self,
        subscriber: SubscriberId,
        scope: &str,
        value: &str,
        until: Option<Timestamp>,
        now: Timestamp,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// Mutes that have not expired at `now`.
    fn mutes(
        &self,
        subscriber: SubscriberId,
        now: Timestamp,
    ) -> impl Future<Output = Result<Vec<MuteEntry>, StoreError>> + Send;

    /// Removes a mute; returns whether it existed.
    fn remove_mute(
        &self,
        subscriber: SubscriberId,
        id: i64,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;
}

/// Why a mute could not be added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuteError {
    /// Unknown scope.
    Scope,
    /// The value does not fit the scope.
    Value,
    /// The duration is not like `7d`.
    Duration(String),
}

/// Mute operations.
pub struct Mutes<S> {
    store: S,
    clock: Arc<dyn Clock>,
}

impl<S: MuteStore> Mutes<S> {
    /// Creates the service.
    pub fn new(store: S, clock: Arc<dyn Clock>) -> Self {
        Self { store, clock }
    }

    /// `/mutes add <scope> <value> [duration]`; returns when it ends.
    pub async fn add(
        &self,
        subscriber: SubscriberId,
        scope: &str,
        value: &str,
        duration: Option<&str>,
    ) -> Result<Result<Option<Timestamp>, MuteError>, StoreError> {
        let scope = match scope {
            "version" => "component-version",
            other => other,
        };
        if !SCOPES.contains(&scope) {
            return Ok(Err(MuteError::Scope));
        }
        let valid = match scope {
            "story" => value.parse::<i64>().is_ok(),
            "entity" => value.contains(':') && !value.starts_with("pkg:"),
            "component" => value.starts_with("pkg:") || value.starts_with("cpe:"),
            _ => {
                value.starts_with("pkg:")
                    && value.rsplit_once('@').is_some_and(|(_, v)| !v.is_empty())
            }
        };
        if !valid {
            return Ok(Err(MuteError::Value));
        }
        let now = self.clock.now();
        let until = match duration {
            None => None,
            Some(text) => match parse_duration(text) {
                Ok(d) if d.is_positive() => now.checked_add(d).ok(),
                _ => return Ok(Err(MuteError::Duration(text.to_string()))),
            },
        };
        self.store
            .add_mute(subscriber, scope, value, until, now)
            .await?;
        Ok(Ok(until))
    }

    /// `/mutes`.
    pub async fn list(&self, subscriber: SubscriberId) -> Result<Vec<MuteEntry>, StoreError> {
        self.store.mutes(subscriber, self.clock.now()).await
    }

    /// `/mutes remove <id>`.
    pub async fn remove(&self, subscriber: SubscriberId, id: i64) -> Result<bool, StoreError> {
        self.store.remove_mute(subscriber, id).await
    }
}
