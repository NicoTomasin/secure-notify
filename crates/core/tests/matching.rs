// SPDX-License-Identifier: AGPL-3.0-or-later

//! Relevance, mutes, rules, and the minimum severity (docs/architecture/matching-and-rules.md).
//! The advisory is the real GHSA-35jh-r3h4-6jhm range for lodash (fixed in 4.17.21).

#![allow(clippy::unwrap_used)]

use secure_notify_core::correlation::StoryId;
use secure_notify_core::matching::{
    Candidate, Mute, NoticeKind, Reason, Relevance, Snapshot, StoryFacts, decide, default_rules,
    plan, relevance, update_due,
};
use secure_notify_core::subscribers::SubscriberId;
use secure_notify_model::{
    Affected, Component, EventKind, Range, RangeEvent, RangeType, SeverityLevel,
};

const STORY: StoryId = StoryId(7);

fn lodash() -> Affected {
    Affected {
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
    }
}

fn story(severity: SeverityLevel) -> StoryFacts {
    StoryFacts {
        kind: EventKind::Vulnerability,
        severity,
        exploited: false,
        exploit_available: false,
        fix_available: true,
        epss: None,
        sources: vec!["osv".to_string()],
        entities: vec!["tech:node".to_string()],
        affected: vec![lodash()],
        eligible_at: "2026-11-27T12:05:00Z".parse().unwrap(),
    }
}

fn candidate(entries: &[(&str, Option<&str>)], watched: &[&str]) -> Candidate {
    Candidate {
        subscriber: SubscriberId(1),
        entries: entries
            .iter()
            .map(|(c, v)| ((*c).to_string(), v.map(str::to_string)))
            .collect(),
        watched: watched.iter().map(|w| (*w).to_string()).collect(),
        rules: Vec::new(),
        min_severity: None,
        mutes: Vec::new(),
        delivered: None,
    }
}

#[test]
fn relevance_levels_follow_the_documented_table() {
    let s = story(SeverityLevel::High);
    let cases: [(&str, Candidate, Option<Relevance>); 6] = [
        (
            "affected exact version",
            candidate(&[("pkg:npm/lodash", Some("4.17.20"))], &[]),
            Some(Relevance::ExactVersion),
        ),
        (
            "version unknown",
            candidate(&[("pkg:npm/lodash", None)], &[]),
            Some(Relevance::Component),
        ),
        (
            "version that cannot be evaluated",
            candidate(&[("pkg:npm/lodash", Some("^4.0.0"))], &[]),
            Some(Relevance::Component),
        ),
        // A known version outside the range is not relevant for that component.
        (
            "fixed version",
            candidate(&[("pkg:npm/lodash", Some("4.17.21"))], &[]),
            None,
        ),
        (
            "watched technology",
            candidate(&[], &["tech:node"]),
            Some(Relevance::Technology),
        ),
        (
            "unrelated",
            candidate(&[("pkg:npm/left-pad", Some("1.0.0"))], &["tech:bun"]),
            None,
        ),
    ];
    for (case, c, expected) in cases {
        assert_eq!(
            relevance(STORY, &s, &c).map(|r| r.relevance),
            expected,
            "{case}"
        );
    }
}

#[test]
fn the_highest_relevance_wins() {
    let s = story(SeverityLevel::High);
    let c = candidate(
        &[
            ("pkg:npm/lodash", None),
            ("pkg:npm/lodash", Some("4.17.20")),
        ],
        &["tech:node"],
    );
    let reason = relevance(STORY, &s, &c).unwrap();
    assert_eq!(reason.relevance, Relevance::ExactVersion);
    assert_eq!(reason.entries[0].version.as_deref(), Some("4.17.20"));
    assert_eq!(reason.entries[0].fixed, ["4.17.21"]);
}

#[test]
fn mutes_beat_relevance() {
    let s = story(SeverityLevel::High);
    let cases = [Mute::Story(STORY), Mute::Entity("tech:node".to_string())];
    for mute in cases {
        let mut c = candidate(&[("pkg:npm/lodash", Some("4.17.20"))], &["tech:node"]);
        c.mutes = vec![mute.clone()];
        assert_eq!(relevance(STORY, &s, &c), None, "{mute:?}");
    }
    // Muting the component or the version removes that entry; the watch still applies.
    for mute in [
        Mute::Component("pkg:npm/lodash".to_string()),
        Mute::ComponentVersion("pkg:npm/lodash".to_string(), "4.17.20".to_string()),
    ] {
        let mut c = candidate(&[("pkg:npm/lodash", Some("4.17.20"))], &["tech:node"]);
        c.mutes = vec![mute.clone()];
        assert_eq!(
            relevance(STORY, &s, &c).map(|r| r.relevance),
            Some(Relevance::Technology),
            "{mute:?}"
        );
    }
}

fn reason(level: Relevance) -> Reason {
    Reason {
        relevance: level,
        entries: Vec::new(),
        entities: Vec::new(),
    }
}

#[test]
fn default_rules_follow_the_documented_table_row_by_row() {
    // matching-and-rules.md#default-rules-for-new-subscribers, one case per row (and its
    // threshold boundary).
    let rules = default_rules();
    let mut exploited = story(SeverityLevel::Low);
    exploited.exploited = true;
    let mut breach = story(SeverityLevel::Medium);
    breach.kind = EventKind::Breach;
    let mut outage = story(SeverityLevel::Medium);
    outage.kind = EventKind::Outage;
    let cases = [
        (
            "1 exploited, any severity",
            exploited,
            Relevance::Technology,
            true,
        ),
        (
            "2 identity",
            story(SeverityLevel::Low),
            Relevance::Identity,
            true,
        ),
        (
            "3 exact-version, medium",
            story(SeverityLevel::Medium),
            Relevance::ExactVersion,
            true,
        ),
        (
            "3 exact-version, low",
            story(SeverityLevel::Low),
            Relevance::ExactVersion,
            false,
        ),
        (
            "4 component, high",
            story(SeverityLevel::High),
            Relevance::Component,
            true,
        ),
        (
            "4 component, medium",
            story(SeverityLevel::Medium),
            Relevance::Component,
            false,
        ),
        (
            "5 product, medium",
            story(SeverityLevel::Medium),
            Relevance::Product,
            false,
        ),
        (
            "6 organization breach, medium",
            breach,
            Relevance::Organization,
            true,
        ),
        (
            "7 organization outage, medium",
            outage,
            Relevance::Organization,
            false,
        ),
        (
            "8 technology, high",
            story(SeverityLevel::High),
            Relevance::Technology,
            true,
        ),
        (
            "8 technology, medium",
            story(SeverityLevel::Medium),
            Relevance::Technology,
            false,
        ),
    ];
    assert_eq!(rules.len(), 8);
    for (case, s, level, expected) in cases {
        assert_eq!(decide(&rules, None, &s, &reason(level)), expected, "{case}");
    }
}

#[test]
fn unknown_severity_passes_medium_but_not_high_thresholds() {
    let rules = default_rules();
    let s = story(SeverityLevel::Unknown);
    assert!(decide(&rules, None, &s, &reason(Relevance::ExactVersion)));
    assert!(!decide(&rules, None, &s, &reason(Relevance::Component)));
}

#[test]
fn the_minimum_severity_applies_after_rules_except_to_exploited_stories() {
    // matching-and-rules.md#minimum-severity (Decided).
    let rules = default_rules();
    let s = story(SeverityLevel::Medium);
    assert!(decide(&rules, None, &s, &reason(Relevance::ExactVersion)));
    assert!(!decide(
        &rules,
        Some(SeverityLevel::High),
        &s,
        &reason(Relevance::ExactVersion)
    ));
    let mut exploited = story(SeverityLevel::Low);
    exploited.exploited = true;
    assert!(decide(
        &rules,
        Some(SeverityLevel::Critical),
        &exploited,
        &reason(Relevance::Technology)
    ));
}

#[test]
fn update_notices_follow_the_fixed_mvp_triggers() {
    // deduplication.md#update-notices and roadmap.md: severity increase, exploited, exploit
    // available, fix available.
    let sent = Snapshot {
        severity: SeverityLevel::Medium,
        exploited: false,
        exploit_available: false,
        fix_available: false,
    };
    let cases = [
        ("same", sent, false),
        (
            "severity up",
            Snapshot {
                severity: SeverityLevel::High,
                ..sent
            },
            true,
        ),
        (
            "severity down",
            Snapshot {
                severity: SeverityLevel::Low,
                ..sent
            },
            false,
        ),
        (
            "exploited",
            Snapshot {
                exploited: true,
                ..sent
            },
            true,
        ),
        (
            "exploit available",
            Snapshot {
                exploit_available: true,
                ..sent
            },
            true,
        ),
        (
            "fix available",
            Snapshot {
                fix_available: true,
                ..sent
            },
            true,
        ),
    ];
    for (case, now, expected) in cases {
        assert_eq!(update_due(&sent, &now), expected, "{case}");
    }
}

#[test]
fn delivered_stories_are_queued_again_only_as_updates() {
    let s = story(SeverityLevel::High);
    let now = "2026-11-27T12:00:00Z".parse().unwrap();
    let mut c = candidate(&[("pkg:npm/lodash", Some("4.17.20"))], &[]);
    let first = plan(STORY, &s, &[c.clone()], now);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].kind, NoticeKind::New);
    // Due after the grace window.
    assert_eq!(first[0].due_at.to_string(), "2026-11-27T12:05:00Z");
    c.delivered = Some(s.snapshot());
    assert!(plan(STORY, &s, &[c.clone()], now).is_empty());
    let mut exploited = s.clone();
    exploited.exploited = true;
    let update = plan(STORY, &exploited, &[c], now);
    assert_eq!(update[0].kind, NoticeKind::Update);
}
