// SPDX-License-Identifier: AGPL-3.0-or-later

//! Watched entities (docs/architecture/taxonomy.md, docs/architecture/platforms.md#command-set).

use std::future::Future;
use std::sync::Arc;

use jiff::Timestamp;
use secure_notify_model::TaxonomyEntity;

use crate::ports::{Clock, StoreError};
use crate::subscribers::SubscriberId;
use crate::taxonomy::Taxonomy;

/// How a watch was created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchOrigin {
    /// With `/watch`.
    Manual,
    /// Derived from an upload (docs/architecture/inventory.md#upload-flow).
    Derived,
}

impl WatchOrigin {
    /// The stored name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Derived => "derived",
        }
    }
}

/// Persistence for watches.
pub trait WatchStore: Send + Sync {
    /// Adds a watch. A manual watch replaces a derived one; a derived watch never replaces a
    /// manual one. Returns whether the subscriber already had a manual watch for the entity.
    fn add_watch(
        &self,
        subscriber: SubscriberId,
        entity: &str,
        origin: WatchOrigin,
        now: Timestamp,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// Removes a watch, whatever its origin. Returns whether it existed.
    fn remove_watch(
        &self,
        subscriber: SubscriberId,
        entity: &str,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;

    /// The subscriber's watches, sorted by entity ID.
    fn watches(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<Vec<(String, WatchOrigin)>, StoreError>> + Send;
}

/// Result of `/watch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchOutcome {
    /// Now watched.
    Added(TaxonomyEntity),
    /// Already watched manually.
    AlreadyWatched(TaxonomyEntity),
    /// Not in the taxonomy.
    Unknown,
}

/// Result of `/unwatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnwatchOutcome {
    /// No longer watched.
    Removed(TaxonomyEntity),
    /// It was not watched.
    NotWatched(TaxonomyEntity),
    /// Not in the taxonomy.
    Unknown,
}

/// Watch operations.
pub struct Watches<S> {
    store: S,
    taxonomy: Arc<Taxonomy>,
    clock: Arc<dyn Clock>,
}

impl<S: WatchStore> Watches<S> {
    /// Creates the service.
    pub fn new(store: S, taxonomy: Arc<Taxonomy>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            taxonomy,
            clock,
        }
    }

    /// The taxonomy.
    pub fn taxonomy(&self) -> &Taxonomy {
        &self.taxonomy
    }

    /// `/watch <query>`.
    pub async fn watch(
        &self,
        subscriber: SubscriberId,
        query: &str,
    ) -> Result<WatchOutcome, StoreError> {
        let Some(entity) = self.taxonomy.resolve(query).cloned() else {
            return Ok(WatchOutcome::Unknown);
        };
        let already = self
            .store
            .add_watch(
                subscriber,
                &entity.id,
                WatchOrigin::Manual,
                self.clock.now(),
            )
            .await?;
        Ok(if already {
            WatchOutcome::AlreadyWatched(entity)
        } else {
            WatchOutcome::Added(entity)
        })
    }

    /// Adds watches derived from an upload; existing manual watches are kept.
    pub async fn derive(
        &self,
        subscriber: SubscriberId,
        entities: &[String],
    ) -> Result<(), StoreError> {
        let now = self.clock.now();
        for entity in entities {
            if self.taxonomy.get(entity).is_some() {
                self.store
                    .add_watch(subscriber, entity, WatchOrigin::Derived, now)
                    .await?;
            }
        }
        Ok(())
    }

    /// `/unwatch <query>`.
    pub async fn unwatch(
        &self,
        subscriber: SubscriberId,
        query: &str,
    ) -> Result<UnwatchOutcome, StoreError> {
        let Some(entity) = self.taxonomy.resolve(query).cloned() else {
            return Ok(UnwatchOutcome::Unknown);
        };
        Ok(if self.store.remove_watch(subscriber, &entity.id).await? {
            UnwatchOutcome::Removed(entity)
        } else {
            UnwatchOutcome::NotWatched(entity)
        })
    }

    /// `/watching`: watched entities with their origin. Watches of entities no longer in the
    /// taxonomy are shown by ID.
    pub async fn list(
        &self,
        subscriber: SubscriberId,
    ) -> Result<Vec<(String, String, WatchOrigin)>, StoreError> {
        let watches = self.store.watches(subscriber).await?;
        Ok(watches
            .into_iter()
            .map(|(id, origin)| {
                let name = self
                    .taxonomy
                    .get(&id)
                    .map_or_else(|| id.clone(), |e| e.name.clone());
                (id, name, origin)
            })
            .collect())
    }
}
