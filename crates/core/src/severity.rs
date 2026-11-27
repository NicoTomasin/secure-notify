// SPDX-License-Identifier: AGPL-3.0-or-later

//! Severity normalization and thresholds
//! (docs/architecture/matching-and-rules.md#severity-normalization,
//! docs/architecture/matching-and-rules.md#severity-thresholds).

use secure_notify_model::{
    DataClass, EventKind, IncidentImpact, IncidentStatus, NormalizedEvent, SeverityLevel,
};

/// Rank used to compare levels; `unknown` is the lowest stored level.
fn rank(level: SeverityLevel) -> u8 {
    match level {
        SeverityLevel::Unknown => 0,
        SeverityLevel::Low => 1,
        SeverityLevel::Medium => 2,
        SeverityLevel::High => 3,
        SeverityLevel::Critical => 4,
    }
}

/// The higher of two stored levels (a story's level is the highest among its events).
pub fn max(a: SeverityLevel, b: SeverityLevel) -> SeverityLevel {
    if rank(b) > rank(a) { b } else { a }
}

/// Whether `level` passes a `threshold`; `unknown` counts as `medium` in every comparison.
pub fn passes(level: SeverityLevel, threshold: SeverityLevel) -> bool {
    let effective = |l: SeverityLevel| match l {
        SeverityLevel::Unknown => rank(SeverityLevel::Medium),
        other => rank(other),
    };
    effective(level) >= effective(threshold)
}

/// Whether `new` is a higher stored level than `old` (an update-notice trigger).
pub fn increased(old: SeverityLevel, new: SeverityLevel) -> bool {
    rank(new) > rank(old)
}

fn from_cvss(score: f64) -> SeverityLevel {
    match score {
        s if s >= 9.0 => SeverityLevel::Critical,
        s if s >= 7.0 => SeverityLevel::High,
        s if s >= 4.0 => SeverityLevel::Medium,
        s if s > 0.0 => SeverityLevel::Low,
        _ => SeverityLevel::Unknown,
    }
}

fn breach_level(event: &NormalizedEvent) -> SeverityLevel {
    let Some(breach) = &event.breach else {
        return SeverityLevel::Unknown;
    };
    let class_level = |class: DataClass| match class {
        DataClass::Password
        | DataClass::AuthToken
        | DataClass::PaymentCard
        | DataClass::Financial
        | DataClass::GovernmentId
        | DataClass::Health
        | DataClass::Biometric => SeverityLevel::Critical,
        DataClass::PasswordHash
        | DataClass::SecurityQa
        | DataClass::PrivateMessages
        | DataClass::Location
        | DataClass::DateOfBirth => SeverityLevel::High,
        DataClass::Name | DataClass::Phone | DataClass::Address | DataClass::IpAddress => {
            SeverityLevel::Medium
        }
        DataClass::Email | DataClass::Username | DataClass::Other => SeverityLevel::Low,
    };
    let base = breach
        .data_classes
        .iter()
        .map(|c| class_level(*c))
        .fold(SeverityLevel::Unknown, max);
    let raise = breach.records.is_some_and(|r| r >= 10_000_000);
    match (base, raise) {
        (SeverityLevel::Unknown, _) | (_, false) => base,
        (SeverityLevel::Low, true) => SeverityLevel::Medium,
        (SeverityLevel::Medium, true) => SeverityLevel::High,
        (SeverityLevel::High | SeverityLevel::Critical, true) => SeverityLevel::Critical,
    }
}

fn incident_level(event: &NormalizedEvent) -> SeverityLevel {
    let Some(incident) = &event.incident else {
        return SeverityLevel::Unknown;
    };
    if incident.status == IncidentStatus::Scheduled {
        return SeverityLevel::Low;
    }
    match incident.impact {
        Some(IncidentImpact::Critical) => SeverityLevel::Critical,
        Some(IncidentImpact::Major) => SeverityLevel::High,
        Some(IncidentImpact::Minor) => SeverityLevel::Medium,
        Some(IncidentImpact::None) => SeverityLevel::Low,
        None => SeverityLevel::Unknown,
    }
}

/// The normalized level of one event. An explicit level from the source (including a declarative
/// source's default, which the engine sets as the level) is used as-is.
pub fn normalize(event: &NormalizedEvent) -> SeverityLevel {
    if let Some(level) = event.severity.level {
        return level;
    }
    match event.kind {
        EventKind::MaliciousPackage => SeverityLevel::Critical,
        EventKind::Breach => breach_level(event),
        EventKind::Attack | EventKind::Outage => incident_level(event),
        _ => event
            .severity
            .cvss
            .as_ref()
            .map_or(SeverityLevel::Unknown, |c| from_cvss(c.score)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secure_notify_model::{Breach, Cvss, Incident};

    fn event(kind: &str) -> NormalizedEvent {
        serde_json::from_value(serde_json::json!({
            "schema_version": "1", "source": "s", "source_item_id": "1", "title": "t", "kind": kind
        }))
        .unwrap()
    }

    fn with_cvss(score: f64) -> NormalizedEvent {
        let mut e = event("vulnerability");
        e.severity.cvss = Some(Cvss {
            version: "3.1".to_string(),
            score,
            vector: None,
        });
        e
    }

    #[test]
    fn vulnerability_family_follows_the_documented_table() {
        let mut explicit = with_cvss(9.8);
        explicit.severity.level = Some(SeverityLevel::Low);
        let cases = [
            ("explicit level used as-is", explicit, SeverityLevel::Low),
            (
                "malicious package without level",
                event("malicious-package"),
                SeverityLevel::Critical,
            ),
            ("CVSS 9.0", with_cvss(9.0), SeverityLevel::Critical),
            ("CVSS 8.9", with_cvss(8.9), SeverityLevel::High),
            ("CVSS 7.0", with_cvss(7.0), SeverityLevel::High),
            ("CVSS 6.9", with_cvss(6.9), SeverityLevel::Medium),
            ("CVSS 4.0", with_cvss(4.0), SeverityLevel::Medium),
            ("CVSS 3.9", with_cvss(3.9), SeverityLevel::Low),
            ("CVSS 0.1", with_cvss(0.1), SeverityLevel::Low),
            ("neither", event("vulnerability"), SeverityLevel::Unknown),
        ];
        for (case, e, expected) in cases {
            assert_eq!(normalize(&e), expected, "{case}");
        }
    }

    #[test]
    fn breaches_and_incidents_follow_the_documented_tables() {
        let breach = |classes: Vec<DataClass>, records: Option<u64>| {
            let mut e = event("breach");
            e.breach = Some(Breach {
                organization: None,
                disclosed_at: None,
                occurred_at: None,
                records,
                data_classes: classes,
            });
            e
        };
        let incident = |status: IncidentStatus, impact: Option<IncidentImpact>| {
            let mut e = event("outage");
            e.incident = Some(Incident {
                organization: None,
                status,
                impact,
                started_at: None,
                resolved_at: None,
            });
            e
        };
        let cases = [
            (
                breach(vec![DataClass::Email, DataClass::Password], None),
                SeverityLevel::Critical,
            ),
            (
                breach(vec![DataClass::PasswordHash], None),
                SeverityLevel::High,
            ),
            (breach(vec![DataClass::Phone], None), SeverityLevel::Medium),
            (breach(vec![DataClass::Email], None), SeverityLevel::Low),
            (
                breach(vec![DataClass::Email], Some(10_000_000)),
                SeverityLevel::Medium,
            ),
            (
                breach(vec![DataClass::PasswordHash], Some(20_000_000)),
                SeverityLevel::Critical,
            ),
            (breach(vec![], Some(50_000_000)), SeverityLevel::Unknown),
            (
                incident(IncidentStatus::Investigating, Some(IncidentImpact::Major)),
                SeverityLevel::High,
            ),
            (
                incident(IncidentStatus::Investigating, Some(IncidentImpact::None)),
                SeverityLevel::Low,
            ),
            (
                incident(IncidentStatus::Investigating, None),
                SeverityLevel::Unknown,
            ),
            (
                incident(IncidentStatus::Scheduled, Some(IncidentImpact::Critical)),
                SeverityLevel::Low,
            ),
        ];
        for (i, (e, expected)) in cases.into_iter().enumerate() {
            assert_eq!(normalize(&e), expected, "case {i}");
        }
    }

    #[test]
    fn unknown_counts_as_medium_in_thresholds() {
        // matching-and-rules.md#severity-thresholds (Decided).
        assert!(passes(SeverityLevel::Unknown, SeverityLevel::Medium));
        assert!(!passes(SeverityLevel::Unknown, SeverityLevel::High));
        assert!(passes(SeverityLevel::High, SeverityLevel::High));
        assert!(!passes(SeverityLevel::Low, SeverityLevel::Medium));
    }
}
