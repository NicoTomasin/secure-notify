// SPDX-License-Identifier: AGPL-3.0-or-later

//! Mirror of the npm ecosystem of OSV (docs/architecture/inventory.md#inventory-stays-local).
//!
//! The first synchronization (a backfill) streams `npm/all.zip` through a temporary file and
//! yields its records in batches, so memory stays within the bulk synchronization budget. Later
//! runs read `npm/modified_id.csv` (newest first) and fetch only the records modified since the
//! cursor; records with `withdrawn` are reported as withdrawn and removed. Aliases (CVE, GHSA,
//! `MAL-`) become identifiers.

use std::io::{BufRead, BufReader, Read};

use jiff::Timestamp;
use secure_notify_core::ports::HttpRequest;
use secure_notify_core::sources::{
    Batches, BoxFuture, FetchOutput, RawItem, Source, SourceError, SourceHttp, SourceInfo,
    SourceState,
};
use secure_notify_model::{
    Affected, Component, EventKind, Identifiers, NormalizedEvent, Range, RangeEvent, RangeType,
    SCHEMA_VERSION, Severity, SeverityLevel, npm_purl,
};
use serde_json::Value;

/// The OSV bucket.
const BASE: &str = "https://osv-vulnerabilities.storage.googleapis.com/npm";
/// State key: modification time of the newest record already synchronized.
const CURSOR: &str = "cursor";
/// Records per batch during the bulk synchronization.
const BATCH: usize = 500;
/// More changed records than this since the cursor are synchronized from `all.zip` again.
const MAX_INCREMENTAL: usize = 2000;
/// Largest accepted `all.zip` and `modified_id.csv`.
const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_INDEX_BYTES: u64 = 200 * 1024 * 1024;
/// Largest summary kept from a record's details.
const MAX_SUMMARY_CHARS: usize = 1000;
/// Most references kept as links.
const MAX_LINKS: usize = 20;

/// The OSV npm mirror.
pub struct OsvSource {
    info: SourceInfo,
}

impl OsvSource {
    /// Creates the source with its manifest values.
    pub fn new() -> Self {
        Self {
            info: SourceInfo {
                id: "osv".to_string(),
                name: "OSV (npm)".to_string(),
                interval: jiff::SignedDuration::from_hours(1),
                mirror: true,
                ecosystems: vec!["npm".to_string()],
                default_enabled: true,
                structured: true,
                hosts: vec!["osv-vulnerabilities.storage.googleapis.com".to_string()],
            },
        }
    }
}

impl Default for OsvSource {
    fn default() -> Self {
        Self::new()
    }
}

fn response_error(message: impl Into<String>) -> SourceError {
    SourceError::Response(message.into())
}

/// A record as a raw item, or `None` for content that is not an OSV record.
fn raw_item(data: Value) -> Option<RawItem> {
    let id = data.get("id")?.as_str()?.to_string();
    Some(RawItem { id, data })
}

fn is_withdrawn(data: &Value) -> bool {
    data.get("withdrawn").is_some_and(|w| !w.is_null())
}

/// Reads the records of `all.zip` in batches. Withdrawn records are skipped: nothing of them is
/// stored yet.
struct ZipBatches {
    archive: zip::ZipArchive<std::fs::File>,
    next: usize,
}

impl Iterator for ZipBatches {
    type Item = Result<Vec<RawItem>, SourceError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.archive.len() {
            return None;
        }
        let mut items = Vec::with_capacity(BATCH);
        while self.next < self.archive.len() && items.len() < BATCH {
            let index = self.next;
            self.next += 1;
            let mut entry = match self.archive.by_index(index) {
                Ok(entry) => entry,
                Err(e) => return Some(Err(response_error(format!("bad archive entry: {e}")))),
            };
            if !entry.name().is_ok_and(|n| n.ends_with(".json")) {
                continue;
            }
            let mut text = String::new();
            if let Err(e) = entry.read_to_string(&mut text) {
                return Some(Err(response_error(format!(
                    "cannot read archive entry: {e}"
                ))));
            }
            match serde_json::from_str::<Value>(&text) {
                Ok(data) if !is_withdrawn(&data) => items.extend(raw_item(data)),
                Ok(_) => {}
                Err(e) => return Some(Err(response_error(format!("record is not JSON: {e}")))),
            }
        }
        Some(Ok(items))
    }
}

impl OsvSource {
    async fn bulk(
        &self,
        http: &dyn SourceHttp,
        state: &mut SourceState,
    ) -> Result<FetchOutput, SourceError> {
        let started = Timestamp::now();
        let download = http
            .download(
                HttpRequest {
                    url: format!("{BASE}/all.zip"),
                    headers: Vec::new(),
                },
                MAX_ARCHIVE_BYTES,
            )
            .await?;
        if download.status != 200 {
            return Err(response_error(format!("all.zip: HTTP {}", download.status)));
        }
        // Records modified after the archive was written are newer than its Last-Modified time.
        let cursor = download
            .header("last-modified")
            .and_then(|d| jiff::fmt::rfc2822::parse(d).ok())
            .map_or(started, |z| z.timestamp());
        let archive = zip::ZipArchive::new(download.file)
            .map_err(|e| response_error(format!("all.zip is not a zip archive: {e}")))?;
        state.insert(CURSOR.to_string(), cursor.to_string());
        let batches: Batches = Box::new(ZipBatches { archive, next: 0 });
        Ok(FetchOutput {
            batches,
            withdrawn: Vec::new(),
        })
    }

    async fn incremental(
        &self,
        http: &dyn SourceHttp,
        state: &mut SourceState,
        cursor: Timestamp,
    ) -> Result<FetchOutput, SourceError> {
        let index = http
            .download(
                HttpRequest {
                    url: format!("{BASE}/modified_id.csv"),
                    headers: Vec::new(),
                },
                MAX_INDEX_BYTES,
            )
            .await?;
        if index.status != 200 {
            return Err(response_error(format!(
                "modified_id.csv: HTTP {}",
                index.status
            )));
        }
        // Lines are `<modified>,<id>`, newest first; stop at the cursor.
        let mut changed = Vec::new();
        let mut newest = cursor;
        for line in BufReader::new(index.file).lines() {
            let line = line.map_err(|e| response_error(format!("modified_id.csv: {e}")))?;
            let Some((modified, id)) = line.split_once(',') else {
                continue;
            };
            let Ok(modified) = modified.trim().parse::<Timestamp>() else {
                return Err(response_error(format!(
                    "modified_id.csv: bad time `{modified}`"
                )));
            };
            if modified <= cursor {
                break;
            }
            newest = newest.max(modified);
            changed.push(id.trim().to_string());
            if changed.len() > MAX_INCREMENTAL {
                return self.bulk(http, state).await;
            }
        }
        let mut items = Vec::with_capacity(changed.len());
        let mut withdrawn = Vec::new();
        for id in changed {
            let response = http
                .get(HttpRequest {
                    url: format!("{BASE}/{id}.json"),
                    headers: Vec::new(),
                })
                .await?;
            if response.status == 404 {
                withdrawn.push(id);
                continue;
            }
            if response.status != 200 {
                return Err(response_error(format!("{id}: HTTP {}", response.status)));
            }
            let data: Value = serde_json::from_slice(&response.body)
                .map_err(|e| response_error(format!("{id}: not JSON: {e}")))?;
            if is_withdrawn(&data) {
                withdrawn.push(id);
            } else {
                items.extend(raw_item(data));
            }
        }
        state.insert(CURSOR.to_string(), newest.to_string());
        let mut output = FetchOutput::items(items);
        output.withdrawn = withdrawn;
        Ok(output)
    }
}

impl Source for OsvSource {
    fn info(&self) -> &SourceInfo {
        &self.info
    }

    fn fetch<'a>(
        &'a self,
        http: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<FetchOutput, SourceError>> {
        Box::pin(async move {
            match state.get(CURSOR).and_then(|c| c.parse::<Timestamp>().ok()) {
                Some(cursor) => self.incremental(http, state, cursor).await,
                None => self.bulk(http, state).await,
            }
        })
    }

    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, String> {
        normalize(&self.info.id, &item.data)
    }
}

fn text<'a>(data: &'a Value, key: &str) -> Option<&'a str> {
    data.get(key).and_then(Value::as_str)
}

/// The record's details without OSV's per-source boilerplate, as one line.
fn summary(details: &str) -> Option<String> {
    let kept: Vec<&str> = details
        .lines()
        .map(str::trim)
        .filter(|l| !(l.starts_with("---") || l.starts_with("_-=") || l.starts_with("## Source:")))
        .collect();
    let joined = kept.join(" ");
    let collapsed = joined.split_whitespace().collect::<Vec<_>>().join(" ");
    (!collapsed.is_empty()).then(|| collapsed.chars().take(MAX_SUMMARY_CHARS).collect())
}

fn range_event(event: &Value) -> Option<RangeEvent> {
    let object = event.as_object()?;
    let (key, value) = object.iter().next()?;
    let value = value.as_str()?.to_string();
    match key.as_str() {
        "introduced" => Some(RangeEvent::Introduced(value)),
        "fixed" => Some(RangeEvent::Fixed(value)),
        "last_affected" => Some(RangeEvent::LastAffected(value)),
        "limit" => Some(RangeEvent::Limit(value)),
        _ => None,
    }
}

fn affected_entry(entry: &Value) -> Option<Affected> {
    let package = entry.get("package")?;
    if text(package, "ecosystem") != Some("npm") {
        return None;
    }
    let name = text(package, "name")?;
    let mut ranges = Vec::new();
    let mut fixed = Vec::new();
    for range in entry
        .get("ranges")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let kind = match text(range, "type") {
            Some("SEMVER") => RangeType::Semver,
            Some("ECOSYSTEM") => RangeType::Ecosystem,
            Some("GIT") => RangeType::Git,
            _ => continue,
        };
        let events: Vec<RangeEvent> = range
            .get("events")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(range_event)
            .collect();
        for event in &events {
            if let RangeEvent::Fixed(version) = event
                && !fixed.contains(version)
            {
                fixed.push(version.clone());
            }
        }
        if !events.is_empty() {
            ranges.push(Range { kind, events });
        }
    }
    // Explicit version lists become one-version ranges.
    for version in entry
        .get("versions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(version) = version.as_str() {
            ranges.push(Range {
                kind: RangeType::Semver,
                events: vec![
                    RangeEvent::Introduced(version.to_string()),
                    RangeEvent::LastAffected(version.to_string()),
                ],
            });
        }
    }
    Some(Affected {
        component: Component {
            purl: Some(npm_purl(name)),
            cpe: None,
        },
        ranges,
        fixed,
    })
}

/// Maps one OSV record to a normalized event; a pure function of the record.
pub fn normalize(source: &str, data: &Value) -> Result<NormalizedEvent, String> {
    let id = text(data, "id").ok_or("record without id")?;
    let mut identifiers = Identifiers::default();
    let aliases = data.get("aliases").and_then(Value::as_array);
    let all_ids =
        std::iter::once(id).chain(aliases.into_iter().flatten().filter_map(Value::as_str));
    for value in all_ids {
        let list = if value.starts_with("CVE-") {
            &mut identifiers.cve
        } else if value.starts_with("GHSA-") {
            &mut identifiers.ghsa
        } else {
            &mut identifiers.osv
        };
        if !list.iter().any(|v| v == value) {
            list.push(value.to_string());
        }
    }
    let affected: Vec<Affected> = data
        .get("affected")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(affected_entry)
        .collect();
    if affected.is_empty() {
        return Err(format!("{id} has no npm package"));
    }
    let level = data
        .get("database_specific")
        .and_then(|d| text(d, "severity"))
        .and_then(|s| match s {
            "CRITICAL" => Some(SeverityLevel::Critical),
            "HIGH" => Some(SeverityLevel::High),
            "MODERATE" => Some(SeverityLevel::Medium),
            "LOW" => Some(SeverityLevel::Low),
            _ => None,
        });
    let links: Vec<String> = data
        .get("references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|r| text(r, "url"))
        .filter(|u| u.starts_with("https://") || u.starts_with("http://"))
        .filter(|u| !u.chars().any(char::is_whitespace))
        .take(MAX_LINKS)
        .map(str::to_string)
        .collect();
    let title = text(data, "summary")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(id)
        .to_string();
    let time = |key: &str| text(data, key).and_then(|t| t.parse::<Timestamp>().ok());
    Ok(NormalizedEvent {
        schema_version: SCHEMA_VERSION.to_string(),
        source: source.to_string(),
        source_item_id: id.to_string(),
        url: Some(format!("https://osv.dev/vulnerability/{id}")),
        title,
        summary: text(data, "details").and_then(summary),
        published_at: time("published"),
        updated_at: time("modified"),
        kind: if id.starts_with("MAL-") {
            EventKind::MaliciousPackage
        } else {
            EventKind::Vulnerability
        },
        identifiers,
        links,
        affected,
        entities: secure_notify_model::Entities::default(),
        severity: Severity {
            level,
            ..Severity::default()
        },
        breach: None,
        incident: None,
    })
}
