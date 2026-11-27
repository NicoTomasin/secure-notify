// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validation of normalized events against the field rules of
//! docs/architecture/data-model.md#normalized-event. The JSON Schema describes the shape; these
//! checks add the rules a schema cannot express well (identifier formats, exactly one of `purl`
//! or `cpe`, kind-specific blocks).

use std::fmt;

use crate::event::{Affected, EventKind, NormalizedEvent, SCHEMA_VERSION};

/// Maximum length of `source` and of each identifier.
const MAX_ID_LEN: usize = 256;
/// Maximum length of `source_item_id` and of URLs.
const MAX_LONG_LEN: usize = 2048;

/// Why an event was rejected.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// The input is not JSON of the normalized event's shape.
    #[error("not a normalized event: {0}")]
    Shape(String),
    /// The input has the right shape but breaks field rules.
    #[error("invalid normalized event: {}", join(.0))]
    Rules(Vec<FieldError>),
}

/// A broken field rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// Path of the field, such as `affected[0].component`.
    pub field: String,
    /// What is wrong with it.
    pub reason: String,
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.reason)
    }
}

fn join(errors: &[FieldError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// Parses and validates a normalized event from JSON. This is the entry point the normalizer
/// uses for events produced by sources.
pub fn parse_event(json: &[u8]) -> Result<NormalizedEvent, ValidationError> {
    let event: NormalizedEvent =
        serde_json::from_slice(json).map_err(|e| ValidationError::Shape(e.to_string()))?;
    event.validate()?;
    Ok(event)
}

impl NormalizedEvent {
    /// Checks the field rules.
    pub fn validate(&self) -> Result<(), ValidationError> {
        let mut errors = Errors::default();
        if self.schema_version != SCHEMA_VERSION {
            errors.add(
                "schema_version",
                format!(
                    "unsupported version `{}`, expected `{SCHEMA_VERSION}`",
                    self.schema_version
                ),
            );
        }
        check_token(&mut errors, "source", &self.source, MAX_ID_LEN);
        check_token(
            &mut errors,
            "source_item_id",
            &self.source_item_id,
            MAX_LONG_LEN,
        );
        if self.title.trim().is_empty() {
            errors.add("title", "must not be empty");
        }
        if let Some(url) = &self.url {
            check_url(&mut errors, "url", url);
        }
        for (i, link) in self.links.iter().enumerate() {
            check_url(&mut errors, &format!("links[{i}]"), link);
        }
        self.check_identifiers(&mut errors);
        for (i, affected) in self.affected.iter().enumerate() {
            check_affected(&mut errors, &format!("affected[{i}]"), affected);
        }
        self.check_entities(&mut errors);
        self.check_severity(&mut errors);
        self.check_kind_blocks(&mut errors);
        errors.finish()
    }

    fn check_identifiers(&self, errors: &mut Errors) {
        let ids = &self.identifiers;
        for (i, cve) in ids.cve.iter().enumerate() {
            if !is_cve(cve) {
                errors.add(
                    format!("identifiers.cve[{i}]"),
                    format!("`{cve}` is not a CVE ID"),
                );
            }
        }
        for (i, ghsa) in ids.ghsa.iter().enumerate() {
            if !is_ghsa(ghsa) {
                errors.add(
                    format!("identifiers.ghsa[{i}]"),
                    format!("`{ghsa}` is not a GHSA ID"),
                );
            }
        }
        for (i, osv) in ids.osv.iter().enumerate() {
            let field = format!("identifiers.osv[{i}]");
            if is_cve(osv) || is_ghsa(osv) {
                errors.add(field, "CVE and GHSA IDs belong in `cve` and `ghsa`");
            } else {
                check_token(errors, &field, osv, MAX_ID_LEN);
            }
        }
        for (i, other) in ids.other.iter().enumerate() {
            let field = format!("identifiers.other[{i}]");
            match other.split_once(':') {
                Some((ns, value))
                    if !ns.is_empty()
                        && !value.is_empty()
                        && ns
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') =>
                {
                    check_token(errors, &field, other, MAX_ID_LEN);
                }
                _ => errors.add(
                    field,
                    format!("`{other}` must be namespaced, as `namespace:id`"),
                ),
            }
        }
    }

    fn check_entities(&self, errors: &mut Errors) {
        let lists = [
            ("technologies", "tech:", &self.entities.technologies),
            ("products", "product:", &self.entities.products),
            ("vendors", "vendor:", &self.entities.vendors),
            ("organizations", "org:", &self.entities.organizations),
        ];
        for (name, prefix, ids) in lists {
            for (i, id) in ids.iter().enumerate() {
                if !is_entity_id(id, prefix) {
                    errors.add(
                        format!("entities.{name}[{i}]"),
                        format!("`{id}` is not an ID of the form `{prefix}<name>`"),
                    );
                }
            }
        }
    }

    fn check_severity(&self, errors: &mut Errors) {
        if let Some(cvss) = &self.severity.cvss {
            if !["4.0", "3.1", "3.0", "2.0"].contains(&cvss.version.as_str()) {
                errors.add(
                    "severity.cvss.version",
                    format!("unknown CVSS version `{}`", cvss.version),
                );
            }
            if !(0.0..=10.0).contains(&cvss.score) {
                errors.add("severity.cvss.score", "must be between 0.0 and 10.0");
            }
        }
        if let Some(epss) = &self.severity.epss {
            if !(0.0..=1.0).contains(&epss.score) {
                errors.add("severity.epss.score", "must be between 0.0 and 1.0");
            }
            if !(0.0..=1.0).contains(&epss.percentile) {
                errors.add("severity.epss.percentile", "must be between 0.0 and 1.0");
            }
        }
    }

    fn check_kind_blocks(&self, errors: &mut Errors) {
        if let Some(breach) = &self.breach {
            if self.kind != EventKind::Breach {
                errors.add("breach", "only `breach` events carry a `breach` block");
            }
            if let Some(org) = &breach.organization
                && !is_entity_id(org, "org:")
            {
                errors.add(
                    "breach.organization",
                    format!("`{org}` is not an `org:` ID"),
                );
            }
        }
        if let Some(incident) = &self.incident {
            if !matches!(self.kind, EventKind::Attack | EventKind::Outage) {
                errors.add(
                    "incident",
                    "only `attack` and `outage` events carry an `incident` block",
                );
            }
            if let Some(org) = &incident.organization
                && !is_entity_id(org, "org:")
            {
                errors.add(
                    "incident.organization",
                    format!("`{org}` is not an `org:` ID"),
                );
            }
        }
    }
}

fn check_affected(errors: &mut Errors, field: &str, affected: &Affected) {
    let component = &affected.component;
    match (&component.purl, &component.cpe) {
        (Some(purl), None) => {
            if let Err(reason) = check_purl(purl) {
                errors.add(format!("{field}.component.purl"), reason);
            }
        }
        (None, Some(cpe)) => {
            if let Err(reason) = check_cpe(cpe) {
                errors.add(format!("{field}.component.cpe"), reason);
            }
        }
        _ => errors.add(
            format!("{field}.component"),
            "must have exactly one of `purl` or `cpe`",
        ),
    }
    for (i, range) in affected.ranges.iter().enumerate() {
        if range.events.is_empty() {
            errors.add(format!("{field}.ranges[{i}].events"), "must not be empty");
        }
    }
}

/// A purl without version: `pkg:<type>/<name>`, where an `@` in a name is percent-encoded.
pub(crate) fn check_purl(purl: &str) -> Result<(), String> {
    let rest = purl
        .strip_prefix("pkg:")
        .ok_or_else(|| format!("`{purl}` is not a purl (`pkg:<type>/<name>`)"))?;
    let path = rest.split(['?', '#']).next().unwrap_or(rest);
    let Some((purl_type, name)) = path.split_once('/') else {
        return Err(format!("`{purl}` is not a purl (`pkg:<type>/<name>`)"));
    };
    let type_ok = !purl_type.is_empty()
        && purl_type
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'));
    if !type_ok || name.trim_matches('/').is_empty() || purl.chars().any(char::is_whitespace) {
        return Err(format!("`{purl}` is not a purl (`pkg:<type>/<name>`)"));
    }
    if path.contains('@') {
        return Err(format!("`{purl}` must not contain a version"));
    }
    Ok(())
}

/// A CPE 2.3 formatted string with the version part as `*`.
fn check_cpe(cpe: &str) -> Result<(), String> {
    let parts = split_cpe(cpe);
    if parts.len() != 13 || parts[0] != "cpe" || parts[1] != "2.3" {
        return Err(format!("`{cpe}` is not a CPE 2.3 formatted string"));
    }
    if parts[5] != "*" {
        return Err(format!("`{cpe}` must have `*` as its version part"));
    }
    Ok(())
}

/// Splits a CPE 2.3 formatted string on unescaped colons.
fn split_cpe(cpe: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut escaped = false;
    for c in cpe.chars() {
        match (escaped, c) {
            (false, '\\') => {
                escaped = true;
                if let Some(last) = parts.last_mut() {
                    last.push(c);
                }
            }
            (false, ':') => parts.push(String::new()),
            _ => {
                escaped = false;
                if let Some(last) = parts.last_mut() {
                    last.push(c);
                }
            }
        }
    }
    parts
}

fn is_cve(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("CVE-") else {
        return false;
    };
    let Some((year, number)) = rest.split_once('-') else {
        return false;
    };
    year.len() == 4
        && year.chars().all(|c| c.is_ascii_digit())
        && number.len() >= 4
        && number.chars().all(|c| c.is_ascii_digit())
}

fn is_ghsa(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("GHSA-") else {
        return false;
    };
    let groups: Vec<&str> = rest.split('-').collect();
    groups.len() == 3
        && groups.iter().all(|g| {
            g.len() == 4
                && g.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// `<prefix><name>` where the name is lowercase ASCII letters, digits, `-`, `.`, or `_`.
fn is_entity_id(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|name| {
        !name.is_empty()
            && name.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_')
            })
    })
}

fn check_token(errors: &mut Errors, field: &str, value: &str, max: usize) {
    if value.is_empty() {
        errors.add(field, "must not be empty");
    } else if value.len() > max {
        errors.add(field, format!("must be at most {max} bytes"));
    } else if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        errors.add(field, "must not contain whitespace or control characters");
    }
}

fn check_url(errors: &mut Errors, field: &str, url: &str) {
    let lower = url.to_ascii_lowercase();
    let scheme_ok = ["http://", "https://"]
        .iter()
        .any(|s| lower.starts_with(s) && lower.len() > s.len());
    if !scheme_ok {
        errors.add(field, "must be an `http` or `https` URL");
    } else if url.len() > MAX_LONG_LEN {
        errors.add(field, format!("must be at most {MAX_LONG_LEN} bytes"));
    } else if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        errors.add(field, "must not contain whitespace or control characters");
    }
}

#[derive(Default)]
struct Errors(Vec<FieldError>);

impl Errors {
    fn add(&mut self, field: impl Into<String>, reason: impl Into<String>) {
        self.0.push(FieldError {
            field: field.into(),
            reason: reason.into(),
        });
    }

    fn finish(self) -> Result<(), ValidationError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::Rules(self.0))
        }
    }
}
