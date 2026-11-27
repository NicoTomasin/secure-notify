// SPDX-License-Identifier: MIT OR Apache-2.0

//! The normalized event (docs/architecture/data-model.md#normalized-event).

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Version of the normalized event schema. Additive changes keep it; breaking changes bump it
/// (docs/engineering/releases.md#versioning).
pub const SCHEMA_VERSION: &str = "1";

/// The one shape every source produces, whatever its origin or runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "Normalized event")]
pub struct NormalizedEvent {
    /// Schema version this event conforms to. Currently `"1"`.
    pub schema_version: String,
    /// Identifier of the source that produced the event.
    pub source: String,
    /// Identifier of the item within its source; stable across fetches and unique together with
    /// `source`.
    pub source_item_id: String,
    /// Canonical URL of the item (`http` or `https`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Title of the item.
    pub title: String,
    /// Short plain-text summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// When the item was first published (RFC 3339, UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<Timestamp>,
    /// When the item was last updated (RFC 3339, UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<Timestamp>,
    /// Event kind (docs/product/scope.md#event-kinds).
    pub kind: EventKind,
    /// Strong identifiers.
    #[serde(default)]
    pub identifiers: Identifiers,
    /// Related URLs (`http` or `https`).
    #[serde(default)]
    pub links: Vec<String>,
    /// Affected components and version ranges.
    #[serde(default)]
    pub affected: Vec<Affected>,
    /// Taxonomy entities the event concerns.
    #[serde(default)]
    pub entities: Entities,
    /// Severity and exploitation signals.
    #[serde(default)]
    pub severity: Severity,
    /// Breach details, for `breach` events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breach: Option<Breach>,
    /// Incident details, for `attack` and `outage` events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incident: Option<Incident>,
}

/// Event kinds (docs/product/scope.md#event-kinds).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum EventKind {
    /// A security flaw in software or hardware.
    Vulnerability,
    /// A public exploit or proof of concept, or a report of active exploitation.
    Exploit,
    /// A package published with malicious code.
    MaliciousPackage,
    /// A security or breaking release announcement.
    Release,
    /// A critical non-security defect.
    Bug,
    /// A confirmed or credibly reported data breach or leak.
    Breach,
    /// A security incident without confirmed data exposure.
    Attack,
    /// A service disruption.
    Outage,
    /// Other relevant security information.
    News,
}

/// Families of kinds; events of different families are never merged into one story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KindFamily {
    /// Vulnerabilities and related kinds.
    Vulnerability,
    /// Breaches, attacks, and outages.
    Incident,
    /// `news`, which belongs to both families.
    Both,
}

impl EventKind {
    /// The family of the kind (docs/product/scope.md#event-kinds).
    pub fn family(self) -> KindFamily {
        match self {
            Self::Vulnerability
            | Self::Exploit
            | Self::MaliciousPackage
            | Self::Release
            | Self::Bug => KindFamily::Vulnerability,
            Self::Breach | Self::Attack | Self::Outage => KindFamily::Incident,
            Self::News => KindFamily::Both,
        }
    }

    /// The kind's name as serialized.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vulnerability => "vulnerability",
            Self::Exploit => "exploit",
            Self::MaliciousPackage => "malicious-package",
            Self::Release => "release",
            Self::Bug => "bug",
            Self::Breach => "breach",
            Self::Attack => "attack",
            Self::Outage => "outage",
            Self::News => "news",
        }
    }

    /// Parses a kind from its serialized name.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == name)
    }

    /// Every kind.
    pub const ALL: [Self; 9] = [
        Self::Vulnerability,
        Self::Exploit,
        Self::MaliciousPackage,
        Self::Release,
        Self::Bug,
        Self::Breach,
        Self::Attack,
        Self::Outage,
        Self::News,
    ];
}

/// Strong identifiers of an event.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Identifiers {
    /// CVE IDs (`CVE-2026-1234`).
    #[serde(default)]
    pub cve: Vec<String>,
    /// GitHub Security Advisory IDs (`GHSA-xxxx-xxxx-xxxx`).
    #[serde(default)]
    pub ghsa: Vec<String>,
    /// OSV IDs that are not CVE or GHSA IDs (`MAL-2026-1`, `PYSEC-2026-1`).
    #[serde(default)]
    pub osv: Vec<String>,
    /// Namespaced IDs (`rustsec:RUSTSEC-2026-0001`, `statuspage:<host>:<incident-id>`).
    #[serde(default)]
    pub other: Vec<String>,
}

impl Identifiers {
    /// Every identifier as `(kind, value)`, where kind is `cve`, `ghsa`, `osv`, or `other`.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        let cve = self.cve.iter().map(|v| ("cve", v.as_str()));
        let ghsa = self.ghsa.iter().map(|v| ("ghsa", v.as_str()));
        let osv = self.osv.iter().map(|v| ("osv", v.as_str()));
        let other = self.other.iter().map(|v| ("other", v.as_str()));
        cve.chain(ghsa).chain(osv).chain(other)
    }

    /// Whether no identifier is present.
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }
}

/// A component affected by an event and the ranges of affected versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Affected {
    /// The affected component.
    pub component: Component,
    /// Ranges of affected versions, as OSV-style events.
    #[serde(default)]
    pub ranges: Vec<Range>,
    /// Versions that fix the problem.
    #[serde(default)]
    pub fixed: Vec<String>,
}

/// A component: exactly one of `purl` (without version) or `cpe` (CPE 2.3, version `*`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Component {
    /// Package URL without version, such as `pkg:npm/lodash`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,
    /// CPE 2.3 name with the version part as `*`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpe: Option<String>,
}

/// A range of affected versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Range {
    /// How versions in the range are ordered.
    #[serde(rename = "type")]
    pub kind: RangeType,
    /// OSV-style range events, in order.
    pub events: Vec<RangeEvent>,
}

/// Ordering of the versions in a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RangeType {
    /// Semantic versioning.
    Semver,
    /// Ordering defined by the purl type.
    Ecosystem,
    /// Generic version ordering for CPEs.
    Cpe,
    /// Git commits; not evaluated against inventories.
    Git,
}

/// An OSV-style range event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RangeEvent {
    /// The version where the problem was introduced (`0` for all earlier versions).
    Introduced(String),
    /// The first version that is no longer affected.
    Fixed(String),
    /// The last version that is affected.
    LastAffected(String),
    /// An upper limit of the range, not itself affected.
    Limit(String),
}

/// Taxonomy entities (docs/architecture/taxonomy.md).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Entities {
    /// Technology IDs (`tech:node`).
    #[serde(default)]
    pub technologies: Vec<String>,
    /// Product IDs (`product:...`).
    #[serde(default)]
    pub products: Vec<String>,
    /// Vendor IDs (`vendor:...`).
    #[serde(default)]
    pub vendors: Vec<String>,
    /// Organization IDs (`org:...`).
    #[serde(default)]
    pub organizations: Vec<String>,
}

impl Entities {
    /// Every entity ID.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.technologies
            .iter()
            .chain(&self.products)
            .chain(&self.vendors)
            .chain(&self.organizations)
            .map(String::as_str)
    }
}

/// Severity and exploitation signals.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Severity {
    /// Level; derived when absent (docs/architecture/matching-and-rules.md#severity-normalization).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<SeverityLevel>,
    /// CVSS score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cvss: Option<Cvss>,
    /// EPSS probability, usually set by an enricher.
    #[serde(default)]
    pub epss: Option<Epss>,
    /// Whether the vulnerability is known to be exploited.
    #[serde(default)]
    pub exploited: bool,
    /// Whether a public exploit is available.
    #[serde(default)]
    pub exploit_available: bool,
}

/// Severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SeverityLevel {
    /// Critical.
    Critical,
    /// High.
    High,
    /// Medium.
    Medium,
    /// Low.
    Low,
    /// Not known.
    Unknown,
}

impl SeverityLevel {
    /// The level's name as serialized.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Unknown => "unknown",
        }
    }

    /// Parses a level from its serialized name.
    pub fn parse(name: &str) -> Option<Self> {
        [
            Self::Critical,
            Self::High,
            Self::Medium,
            Self::Low,
            Self::Unknown,
        ]
        .into_iter()
        .find(|l| l.as_str() == name)
    }
}

/// A CVSS score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cvss {
    /// CVSS version: `4.0`, `3.1`, `3.0`, or `2.0`.
    pub version: String,
    /// Base score, 0.0 to 10.0.
    pub score: f64,
    /// Vector string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<String>,
}

/// An EPSS value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Epss {
    /// Probability of exploitation, 0.0 to 1.0.
    pub score: f64,
    /// Percentile, 0.0 to 1.0.
    pub percentile: f64,
}

/// Breach details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Breach {
    /// Organization ID (`org:...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    /// When the breach was disclosed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disclosed_at: Option<Timestamp>,
    /// When the breach occurred.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<Timestamp>,
    /// Number of affected records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub records: Option<u64>,
    /// Classes of exposed data.
    #[serde(default)]
    pub data_classes: Vec<DataClass>,
}

/// Classes of data exposed in a breach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[allow(missing_docs)]
pub enum DataClass {
    Email,
    Username,
    Name,
    Phone,
    Address,
    DateOfBirth,
    IpAddress,
    Location,
    Password,
    PasswordHash,
    SecurityQa,
    AuthToken,
    PrivateMessages,
    Financial,
    PaymentCard,
    GovernmentId,
    Health,
    Biometric,
    Other,
}

/// Incident details, used by `attack` and `outage` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Incident {
    /// Organization ID (`org:...`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    /// Current status.
    pub status: IncidentStatus,
    /// Impact, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact: Option<IncidentImpact>,
    /// When the incident started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<Timestamp>,
    /// When the incident was resolved.
    #[serde(default)]
    pub resolved_at: Option<Timestamp>,
}

/// Incident status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum IncidentStatus {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
    Scheduled,
}

/// Incident impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum IncidentImpact {
    None,
    Minor,
    Major,
    Critical,
}
