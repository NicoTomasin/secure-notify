// SPDX-License-Identifier: AGPL-3.0-or-later

//! The core services, built over one storage adapter.

use std::sync::Arc;

use crate::inventory::{Inventory, InventoryStore, Parser};
use crate::ports::{Clock, InstanceStore, Random};
use crate::rules::{RuleStore, Rules};
use crate::subscribers::{SubscriberStore, Subscribers};
use crate::taxonomy::Taxonomy;
use crate::watches::{WatchStore, Watches};

/// Everything the core needs from storage.
pub trait Store:
    SubscriberStore
    + InstanceStore
    + WatchStore
    + InventoryStore
    + RuleStore
    + Clone
    + Send
    + Sync
    + 'static
{
}

impl<T> Store for T where
    T: SubscriberStore
        + InstanceStore
        + WatchStore
        + InventoryStore
        + RuleStore
        + Clone
        + Send
        + Sync
        + 'static
{
}

/// The core services.
pub struct Core<S> {
    /// Subscribers, pairing, and invites.
    pub subscribers: Subscribers<S>,
    /// Watched entities.
    pub watches: Watches<S>,
    /// Inventories and uploads.
    pub inventory: Inventory<S>,
    /// Rules and the minimum severity.
    pub rules: Rules<S>,
}

impl<S: Store> Core<S> {
    /// Builds the services over `store`.
    pub fn new(
        store: S,
        taxonomy: Arc<Taxonomy>,
        clock: Arc<dyn Clock>,
        random: Arc<dyn Random>,
        parsers: Arc<[Arc<dyn Parser>]>,
    ) -> Self {
        Self {
            subscribers: Subscribers::new(store.clone(), clock.clone(), random),
            inventory: Inventory::new(store.clone(), parsers, taxonomy.clone(), clock.clone()),
            rules: Rules::new(store.clone()),
            watches: Watches::new(store, taxonomy, clock),
        }
    }
}
