// SPDX-License-Identifier: AGPL-3.0-or-later

//! `/rules`: the subscriber's rules and minimum severity
//! (docs/architecture/matching-and-rules.md#rules, #minimum-severity).

use std::future::Future;

use secure_notify_model::SeverityLevel;

use crate::matching::{Rule, default_rules};
use crate::ports::StoreError;
use crate::subscribers::SubscriberId;

/// Persistence for rules.
pub trait RuleStore: Send + Sync {
    /// Custom rules in order; empty when the subscriber uses the defaults.
    fn custom_rules(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<Vec<Rule>, StoreError>> + Send;

    /// Removes custom rules, so the defaults apply again.
    fn reset_rules(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    /// The minimum severity.
    fn min_severity(
        &self,
        subscriber: SubscriberId,
    ) -> impl Future<Output = Result<Option<SeverityLevel>, StoreError>> + Send;

    /// Sets or removes the minimum severity.
    fn set_min_severity(
        &self,
        subscriber: SubscriberId,
        level: Option<SeverityLevel>,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

/// Rule operations.
pub struct Rules<S> {
    store: S,
}

impl<S: RuleStore> Rules<S> {
    /// Creates the service.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// The rules that apply (custom or default), and whether they are the defaults.
    pub async fn effective(
        &self,
        subscriber: SubscriberId,
    ) -> Result<(Vec<Rule>, bool), StoreError> {
        let custom = self.store.custom_rules(subscriber).await?;
        Ok(if custom.is_empty() {
            (default_rules(), true)
        } else {
            (custom, false)
        })
    }

    /// `/rules reset`.
    pub async fn reset(&self, subscriber: SubscriberId) -> Result<(), StoreError> {
        self.store.reset_rules(subscriber).await
    }

    /// The minimum severity.
    pub async fn min_severity(
        &self,
        subscriber: SubscriberId,
    ) -> Result<Option<SeverityLevel>, StoreError> {
        self.store.min_severity(subscriber).await
    }

    /// `/rules severity <level>` and `/rules severity off`. `unknown` is not a threshold.
    pub async fn set_min_severity(
        &self,
        subscriber: SubscriberId,
        level: Option<SeverityLevel>,
    ) -> Result<(), StoreError> {
        self.store.set_min_severity(subscriber, level).await
    }
}
