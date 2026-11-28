// SPDX-License-Identifier: AGPL-3.0-or-later

//! What an initial scan reports (docs/architecture/inventory.md#upload-flow) and which entries
//! wait for a mirror (docs/architecture/inventory.md#initial-scan-before-the-first-synchronization).
//! The advisory is the real GHSA-35jh-r3h4-6jhm range for lodash (fixed in 4.17.21).

#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use secure_notify_core::correlation::StoryId;
use secure_notify_core::delivery::StoryView;
use secure_notify_core::inventory::ParsedComponent;
use secure_notify_core::matching::{Candidate, Relevance, Snapshot, StoryFacts};
use secure_notify_core::scan::{ScanHit, purl_type, report, waits};
use secure_notify_core::subscribers::SubscriberId;
use secure_notify_model::{
    Affected, Component, EventKind, Range, RangeEvent, RangeType, SeverityLevel,
};

fn hit(id: i64, severity: SeverityLevel, exploited: bool) -> ScanHit {
    ScanHit {
        view: StoryView {
            id: StoryId(id),
            kind: EventKind::Vulnerability,
            title: format!("story {id}"),
            url: None,
            severity,
            exploited,
            exploit_available: false,
            fix_available: true,
            cvss: None,
            epss: None,
            summary: None,
            entities: vec!["tech:node".to_string()],
            sources: 1,
        },
        facts: StoryFacts {
            kind: EventKind::Vulnerability,
            severity,
            exploited,
            exploit_available: false,
            fix_available: true,
            epss: None,
            sources: vec!["osv".to_string()],
            entities: vec!["tech:node".to_string()],
            affected: vec![Affected {
                component: Component {
                    purl: Some("pkg:npm/lodash".to_string()),
                    cpe: None,
                },
                ranges: vec![Range {
                    kind: RangeType::Semver,
                    events: vec![
                        RangeEvent::Introduced("0".to_string()),
                        RangeEvent::Fixed("4.17.21".to_string()),
                    ],
                }],
                fixed: vec!["4.17.21".to_string()],
            }],
            eligible_at: "2026-11-27T12:05:00Z".parse().unwrap(),
        },
        // As storage builds it: the whole inventory and the watches.
        candidate: Candidate {
            subscriber: SubscriberId(1),
            entries: vec![
                ("pkg:npm/lodash".to_string(), Some("4.17.20".to_string())),
                ("pkg:npm/lodash".to_string(), Some("4.17.21".to_string())),
            ],
            watched: vec!["tech:node".to_string()],
            rules: Vec::new(),
            min_severity: None,
            mutes: Vec::new(),
            delivered: None,
        },
    }
}

fn entry(version: &str) -> ParsedComponent {
    ParsedComponent {
        purl: "pkg:npm/lodash".to_string(),
        version: Some(version.to_string()),
    }
}

fn ids(hits: Vec<ScanHit>, entries: &[ParsedComponent]) -> Vec<i64> {
    report(entries, hits)
        .stories
        .iter()
        .map(|(s, _)| s.id.0)
        .collect()
}

#[test]
fn stories_are_reported_most_severe_and_exploited_first() {
    let hits = vec![
        hit(1, SeverityLevel::Critical, false),
        hit(2, SeverityLevel::High, false),
        hit(3, SeverityLevel::High, true),
        hit(4, SeverityLevel::High, false),
    ];
    let report = report(&[entry("4.17.20")], hits);
    let order: Vec<i64> = report.stories.iter().map(|(s, _)| s.id.0).collect();
    // Exploited, then by severity, then newest first.
    assert_eq!(order, [3, 1, 4, 2]);
    assert_eq!(report.entries, 1);
    let (_, reason) = &report.stories[0];
    assert_eq!(reason.relevance, Relevance::ExactVersion);
    assert_eq!(reason.entries[0].version.as_deref(), Some("4.17.20"));
}

#[test]
fn stories_already_in_the_ledger_are_left_out() {
    let mut delivered = hit(1, SeverityLevel::High, false);
    delivered.candidate.delivered = Some(Snapshot {
        severity: SeverityLevel::High,
        exploited: false,
        exploit_available: false,
        fix_available: true,
    });
    assert!(ids(vec![delivered], &[entry("4.17.20")]).is_empty());
}

#[test]
fn only_the_scanned_entries_count_not_other_entries_or_watches() {
    // The fixed version was uploaded; the affected one and the watched technology are elsewhere
    // in the subscriber's settings and are matched by the regular pipeline.
    assert!(
        ids(
            vec![hit(1, SeverityLevel::High, false)],
            &[entry("4.17.21")]
        )
        .is_empty()
    );
}

#[test]
fn stories_the_rules_reject_are_left_out() {
    let mut low = hit(1, SeverityLevel::Low, false);
    low.candidate.min_severity = Some(SeverityLevel::High);
    assert!(ids(vec![low], &[entry("4.17.20")]).is_empty());
}

#[test]
fn entries_wait_only_for_mirrors_of_their_ecosystem() {
    assert_eq!(purl_type("pkg:npm/%40octokit/core"), Some("npm"));
    assert_eq!(purl_type("cpe:2.3:a:vendor:product:*"), None);
    let syncing: BTreeSet<String> = ["npm".to_string()].into();
    assert!(waits(&entry("4.17.20"), &syncing));
    let cargo = ParsedComponent {
        purl: "pkg:cargo/serde".to_string(),
        version: None,
    };
    assert!(!waits(&cargo, &syncing));
    assert!(!waits(&entry("4.17.20"), &BTreeSet::new()));
}
