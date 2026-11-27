// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retention and compaction (docs/architecture/deduplication.md#retention): detailed data expires
//! after the retention period, a minimal deduplication record (identifiers, canonical URLs, kind,
//! title, severity snapshot, ledger rows) is kept for the deduplication period, and data from
//! mirrored databases never expires.

use std::future::Future;

use jiff::{SignedDuration, Timestamp};

use crate::ports::StoreError;

/// What one retention pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionCounts {
    /// Event payloads removed.
    pub payloads_expired: usize,
    /// Stories reduced to minimal deduplication records.
    pub stories_compacted: usize,
    /// Minimal records deleted with their ledger rows.
    pub stories_deleted: usize,
}

/// Persistence for retention.
pub trait RetentionStore: Send + Sync {
    /// Removes payloads of non-mirror events unchanged since `details_before`, compacts stories
    /// unchanged since then that hold no mirror event, and deletes compacted stories unchanged
    /// since `records_before`.
    fn compact(
        &self,
        details_before: Timestamp,
        records_before: Timestamp,
    ) -> impl Future<Output = Result<RetentionCounts, StoreError>> + Send;
}

/// Runs one retention pass at `now`.
pub async fn run<S: RetentionStore>(
    store: &S,
    now: Timestamp,
    details: SignedDuration,
    records: SignedDuration,
) -> Result<RetentionCounts, StoreError> {
    let details_before = now
        .checked_sub(details)
        .map_err(|e| StoreError(e.to_string()))?;
    let records_before = now
        .checked_sub(records)
        .map_err(|e| StoreError(e.to_string()))?;
    store.compact(details_before, records_before).await
}
