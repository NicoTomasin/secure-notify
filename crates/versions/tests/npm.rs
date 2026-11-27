// SPDX-License-Identifier: AGPL-3.0-or-later

//! npm version evaluation against real OSV records (test vectors) and properties
//! (docs/engineering/testing.md#test-layers: version ranges).

use proptest::prelude::*;
use secure_notify_model::{Affected, Component, Range, RangeEvent, RangeType};
use secure_notify_versions::{Evaluation, SemVer, evaluate};

fn affected(package: &str, ranges: &[&[(&str, &str)]]) -> Affected {
    Affected {
        component: Component {
            purl: Some(format!("pkg:npm/{package}")),
            cpe: None,
        },
        ranges: ranges
            .iter()
            .map(|events| Range {
                kind: RangeType::Semver,
                events: events
                    .iter()
                    .map(|(kind, version)| {
                        let version = (*version).to_string();
                        match *kind {
                            "introduced" => RangeEvent::Introduced(version),
                            "fixed" => RangeEvent::Fixed(version),
                            "last_affected" => RangeEvent::LastAffected(version),
                            _ => RangeEvent::Limit(version),
                        }
                    })
                    .collect(),
            })
            .collect(),
        fixed: Vec::new(),
    }
}

// Ranges copied from OSV records (`https://api.osv.dev/v1/vulns/<ID>`, retrieved 2026-11-27)
/// and versions on both sides of each boundary.
#[test]
fn real_advisory_ranges_are_evaluated_as_published() {
    use Evaluation::{Affected as A, NotAffected as N};
    let lodash = affected("lodash", &[&[("introduced", "0"), ("fixed", "4.17.21")]]);
    let lodash_template = affected(
        "lodash.template",
        &[&[("introduced", "0"), ("last_affected", "4.5.0")]],
    );
    let minimist = affected(
        "minimist",
        &[
            &[("introduced", "1.0.0"), ("fixed", "1.2.6")],
            &[("introduced", "0"), ("fixed", "0.2.4")],
        ],
    );
    let next = affected(
        "next",
        &[
            &[("introduced", "13.0.0"), ("fixed", "13.5.9")],
            &[("introduced", "14.0.0"), ("fixed", "14.2.25")],
            &[("introduced", "15.0.0"), ("fixed", "15.2.3")],
            &[("introduced", "12.0.0"), ("fixed", "12.3.5")],
        ],
    );
    let cases = [
        // GHSA-35jh-r3h4-6jhm
        (&lodash, "4.17.20", A),
        (&lodash, "0.1.0", A),
        (&lodash, "4.17.21", N),
        (&lodash, "5.0.0", N),
        (&lodash_template, "4.5.0", A),
        (&lodash_template, "4.5.1", N),
        // GHSA-xvch-5gv4-984h
        (&minimist, "0.2.3", A),
        (&minimist, "0.2.4", N),
        (&minimist, "0.9.0", N),
        (&minimist, "1.0.0", A),
        (&minimist, "1.2.5", A),
        (&minimist, "1.2.6", N),
        // GHSA-f82v-jwr5-mffw
        (&next, "11.1.4", N),
        (&next, "12.3.4", A),
        (&next, "13.5.9", N),
        (&next, "14.2.24", A),
        (&next, "14.2.25", N),
        (&next, "15.2.2", A),
        (&next, "15.2.3", N),
        // Pre-releases follow SemVer precedence: 15.2.3-canary.0 < 15.2.3, and
        // 15.0.0-rc.1 < 15.0.0 while above 14.2.25.
        (&next, "15.2.3-canary.0", A),
        (&next, "15.0.0-rc.1", N),
    ];
    for (entry, version, expected) in cases {
        let package = entry.component.purl.as_deref().unwrap();
        assert_eq!(evaluate(entry, version), expected, "{package} {version}");
    }
}

#[test]
fn unparseable_versions_and_unsupported_ranges_are_unknown() {
    // matching-and-rules.md: no reliable evaluation yields `component` relevance, not a guess.
    let lodash = affected("lodash", &[&[("introduced", "0"), ("fixed", "4.17.21")]]);
    assert_eq!(evaluate(&lodash, "^4.17.0"), Evaluation::Unknown);
    assert_eq!(
        evaluate(&lodash, "github:lodash/lodash#master"),
        Evaluation::Unknown
    );
    let mut git = lodash.clone();
    git.ranges[0].kind = RangeType::Git;
    assert_eq!(evaluate(&git, "4.17.20"), Evaluation::Unknown);
    let mut cargo = lodash.clone();
    cargo.component.purl = Some("pkg:cargo/lodash".to_string());
    assert_eq!(evaluate(&cargo, "4.17.20"), Evaluation::Unknown);
    let broken = affected("lodash", &[&[("introduced", "0"), ("fixed", "four")]]);
    assert_eq!(evaluate(&broken, "4.17.20"), Evaluation::Unknown);
    // One range that cannot be read makes the whole entry unknown, even if another range does
    // not contain the version.
    let partly_broken = affected(
        "lodash",
        &[
            &[("introduced", "0"), ("fixed", "4.17.21")],
            &[("introduced", "5.0.0"), ("fixed", "five")],
        ],
    );
    assert_eq!(evaluate(&partly_broken, "4.17.22"), Evaluation::Unknown);
}

#[test]
fn limit_bounds_the_range() {
    let entry = affected("x", &[&[("introduced", "1.0.0"), ("limit", "2.0.0")]]);
    assert_eq!(evaluate(&entry, "1.5.0"), Evaluation::Affected);
    assert_eq!(evaluate(&entry, "2.0.0"), Evaluation::NotAffected);
}

fn version() -> impl Strategy<Value = (u64, u64, u64, Option<u64>)> {
    (0u64..30, 0u64..30, 0u64..30, proptest::option::of(0u64..5))
}

fn text((major, minor, patch, pre): (u64, u64, u64, Option<u64>)) -> String {
    match pre {
        Some(n) => format!("{major}.{minor}.{patch}-rc.{n}"),
        None => format!("{major}.{minor}.{patch}"),
    }
}

proptest! {
    #[test]
    fn printing_and_parsing_round_trip(v in version()) {
        let parsed = SemVer::parse(&text(v)).unwrap();
        prop_assert_eq!(parsed.to_string(), text(v));
    }

    #[test]
    fn releases_order_like_their_numbers(a in version(), b in version()) {
        let (a, b) = ((a.0, a.1, a.2, None), (b.0, b.1, b.2, None));
        let ordering = SemVer::parse(&text(a)).unwrap().cmp(&SemVer::parse(&text(b)).unwrap());
        prop_assert_eq!(ordering, (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    }

    #[test]
    fn pre_releases_come_before_their_release(v in version(), n in 0u64..5) {
        let release = SemVer::parse(&text((v.0, v.1, v.2, None))).unwrap();
        let pre = SemVer::parse(&text((v.0, v.1, v.2, Some(n)))).unwrap();
        prop_assert!(pre < release);
    }

    #[test]
    fn introduced_is_affected_and_fixed_is_not(a in version(), b in version()) {
        let (low, high) = (SemVer::parse(&text(a)).unwrap(), SemVer::parse(&text(b)).unwrap());
        prop_assume!(low < high);
        let entry = affected("x", &[&[("introduced", &text(a)), ("fixed", &text(b))]]);
        prop_assert_eq!(evaluate(&entry, &text(a)), Evaluation::Affected);
        prop_assert_eq!(evaluate(&entry, &text(b)), Evaluation::NotAffected);
    }
}
