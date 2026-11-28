// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source health for `/status` and alerts to admins (docs/operations/self-hosting.md#operations).

#![allow(clippy::unwrap_used, clippy::unused_async_trait_impl)]

use std::sync::{Arc, Mutex};

use secure_notify_core::delivery::{Notice, Outbox};
use secure_notify_core::health::{
    Alerter, HealthStore, InstanceStats, SourceAlert, SourceEntry, SourceHealth, mirrors,
    source_status,
};
use secure_notify_core::ports::{SendError, StoreError};
use secure_notify_core::scan::ScanReport;
use secure_notify_core::sources::{
    BACKFILL_DONE, BoxFuture, FAILURES, ITEMS, LAST_ERROR, LAST_FAILURE, LAST_SUCCESS, SourceState,
};
use secure_notify_core::subscribers::EndpointRef;

fn entry(enabled: bool) -> SourceEntry {
    SourceEntry {
        id: "osv".to_string(),
        name: "OSV (npm)".to_string(),
        enabled,
        mirror: true,
        ecosystems: vec!["npm".to_string()],
    }
}

fn state(pairs: &[(&str, &str)]) -> SourceState {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn a_source_is_disabled_syncing_healthy_or_failing() {
    let healthy = state(&[
        (BACKFILL_DONE, "true"),
        (LAST_SUCCESS, "2026-11-27T10:00:00Z"),
        (FAILURES, "0"),
        (ITEMS, "232471"),
    ]);
    assert_eq!(
        source_status(&entry(false), &healthy).health,
        SourceHealth::Disabled
    );
    let status = source_status(&entry(true), &healthy);
    assert_eq!(
        status.health,
        SourceHealth::Healthy {
            last_success: "2026-11-27T10:00:00Z".parse().unwrap()
        }
    );
    assert_eq!(status.items, 232_471);
    // Never run, or the first synchronization still running.
    assert_eq!(
        source_status(&entry(true), &SourceState::new()).health,
        SourceHealth::FirstSync
    );
    let failing = state(&[
        (BACKFILL_DONE, "true"),
        (LAST_SUCCESS, "2026-11-27T10:00:00Z"),
        (FAILURES, "2"),
        (LAST_FAILURE, "2026-11-27T12:00:00Z"),
        (LAST_ERROR, "unexpected response: HTTP 503"),
    ]);
    assert_eq!(
        source_status(&entry(true), &failing).health,
        SourceHealth::Failing {
            failures: 2,
            last_failure: Some("2026-11-27T12:00:00Z".parse().unwrap()),
            error: "unexpected response: HTTP 503".to_string(),
            last_success: Some("2026-11-27T10:00:00Z".parse().unwrap()),
        }
    );
}

#[test]
fn only_enabled_mirrors_defer_scans() {
    let cisa = SourceEntry {
        id: "cisa-kev".to_string(),
        name: "CISA KEV".to_string(),
        enabled: true,
        mirror: false,
        ecosystems: Vec::new(),
    };
    let found = mirrors(&[entry(true), cisa.clone()]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, "osv");
    assert!(mirrors(&[entry(false), cisa]).is_empty());
}

struct Admins(Vec<(EndpointRef, String)>);

impl HealthStore for Admins {
    async fn instance_stats(&self) -> Result<InstanceStats, StoreError> {
        Ok(InstanceStats::default())
    }
    async fn admin_recipients(&self) -> Result<Vec<(EndpointRef, String)>, StoreError> {
        Ok(self.0.clone())
    }
}

/// Fails for the address `gone`.
#[derive(Default)]
struct Sent(Mutex<Vec<(String, String, SourceAlert)>>);

impl Outbox for Sent {
    fn deliver<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a Notice,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async { Ok(()) })
    }
    fn deliver_report<'a>(
        &'a self,
        _: &'a EndpointRef,
        _: &'a str,
        _: &'a ScanReport,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async { Ok(()) })
    }
    fn deliver_alert<'a>(
        &'a self,
        endpoint: &'a EndpointRef,
        language: &'a str,
        alert: &'a SourceAlert,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            if endpoint.address == "gone" {
                return Err(SendError::EndpointGone);
            }
            self.0.lock().unwrap().push((
                endpoint.address.clone(),
                language.to_string(),
                alert.clone(),
            ));
            Ok(())
        })
    }
}

#[tokio::test]
async fn every_reachable_admin_is_alerted_in_their_language() {
    let admin = |address: &str, language: &str| {
        (
            EndpointRef {
                platform: "test".to_string(),
                address: address.to_string(),
            },
            language.to_string(),
        )
    };
    let store = Admins(vec![
        admin("a", "en"),
        admin("gone", "en"),
        admin("b", "es"),
    ]);
    let outbox = Arc::new(Sent::default());
    let alert = SourceAlert {
        source: "OSV (npm)".to_string(),
        failures: 3,
        error: "unexpected response: HTTP 503".to_string(),
    };
    let reached = Alerter::new(store, outbox.clone())
        .alert(&alert)
        .await
        .unwrap();
    assert_eq!(reached, 2);
    let sent = outbox.0.lock().unwrap();
    let to: Vec<(&str, &str)> = sent
        .iter()
        .map(|(a, l, _)| (a.as_str(), l.as_str()))
        .collect();
    assert_eq!(to, [("a", "en"), ("b", "es")]);
    assert_eq!(sent[0].2, alert);
}
