// SPDX-License-Identifier: AGPL-3.0-or-later

//! Per-ecosystem version and range evaluation for secure-notify
//! (docs/architecture/matching-and-rules.md#version-evaluation).
//!
//! Exact versions from inventories are evaluated locally against OSV-style ranges. Ecosystems
//! and range types without a reliable implementation yield [`Evaluation::Unknown`], which the
//! matcher turns into `component` relevance instead of guessing.

mod semver;

use secure_notify_model::{Affected, RangeEvent, RangeType};

pub use semver::SemVer;

/// Whether a version is inside an affected entry's ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    /// The version is affected.
    Affected,
    /// The version is outside every range.
    NotAffected,
    /// The version or the ranges cannot be evaluated reliably.
    Unknown,
}

/// Version ordering of a purl type, when implemented.
fn ordering_for(purl: &str) -> Option<fn(&str) -> Option<SemVer>> {
    let purl_type = purl.strip_prefix("pkg:")?.split('/').next()?;
    match purl_type {
        "npm" => Some(SemVer::parse),
        _ => None,
    }
}

/// Evaluates `version` against the ranges of `affected` (an entry of an event or story).
pub fn evaluate(affected: &Affected, version: &str) -> Evaluation {
    let Some(parse) = affected.component.purl.as_deref().and_then(ordering_for) else {
        return Evaluation::Unknown;
    };
    let Some(version) = parse(version) else {
        return Evaluation::Unknown;
    };
    let mut evaluated = false;
    for range in &affected.ranges {
        // `semver` ranges and `ecosystem` ranges of npm both use semantic versioning; `git`
        // ranges are not evaluated against inventories (data-model.md).
        if !matches!(range.kind, RangeType::Semver | RangeType::Ecosystem) {
            continue;
        }
        match in_range(&version, &range.events, parse) {
            Some(true) => return Evaluation::Affected,
            Some(false) => evaluated = true,
            None => return Evaluation::Unknown,
        }
    }
    if evaluated {
        Evaluation::NotAffected
    } else {
        Evaluation::Unknown
    }
}

/// The OSV evaluation of one range: events sorted by version; `introduced` opens the range,
/// `fixed` closes it at that version, `last_affected` closes it after that version, and `limit`
/// bounds the whole range. `None` when an event version cannot be parsed.
fn in_range(
    version: &SemVer,
    events: &[RangeEvent],
    parse: fn(&str) -> Option<SemVer>,
) -> Option<bool> {
    let mut sorted = Vec::with_capacity(events.len());
    for event in events {
        let (text, rank) = match event {
            RangeEvent::Introduced(v) => (v, 0),
            RangeEvent::Fixed(v) => (v, 1),
            RangeEvent::LastAffected(v) => (v, 2),
            RangeEvent::Limit(v) => (v, 3),
        };
        let parsed = if text == "0" && rank == 0 {
            SemVer::zero()
        } else {
            parse(text)?
        };
        sorted.push((parsed, rank));
    }
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    if sorted
        .iter()
        .any(|(limit, rank)| *rank == 3 && version >= limit)
    {
        return Some(false);
    }
    let mut affected = false;
    for (event_version, rank) in &sorted {
        match rank {
            0 if version >= event_version => affected = true,
            1 if version >= event_version => affected = false,
            2 if version > event_version => affected = false,
            _ => {}
        }
    }
    Some(affected)
}
