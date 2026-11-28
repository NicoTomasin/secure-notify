// SPDX-License-Identifier: AGPL-3.0-or-later

//! CISA's Known Exploited Vulnerabilities catalog: each entry becomes an `exploit` event with its
//! CVE and `exploited = true`, which sets the `exploited` flag on the story of that CVE
//! (docs/architecture/data-model.md#story).

use jiff::Timestamp;
use secure_notify_core::ports::HttpRequest;
use secure_notify_core::sources::{
    BoxFuture, FetchOutput, RawItem, Source, SourceError, SourceHttp, SourceInfo, SourceState,
};
use secure_notify_model::{EventKind, Identifiers, NormalizedEvent, SCHEMA_VERSION, Severity};
use serde_json::Value;

/// The catalog in JSON.
pub const CATALOG_URL: &str =
    "https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json";
const ETAG: &str = "http.etag";
const LAST_MODIFIED: &str = "http.last_modified";
const MAX_SUMMARY_CHARS: usize = 1000;
const MAX_LINKS: usize = 20;

/// The KEV source.
pub struct KevSource {
    info: SourceInfo,
}

impl KevSource {
    /// Creates the source with its manifest values.
    pub fn new() -> Self {
        Self {
            info: SourceInfo {
                id: "cisa-kev".to_string(),
                name: "CISA Known Exploited Vulnerabilities".to_string(),
                interval: jiff::SignedDuration::from_hours(1),
                mirror: false,
                ecosystems: Vec::new(),
                default_enabled: true,
                structured: true,
                hosts: vec!["www.cisa.gov".to_string()],
            },
        }
    }
}

impl Default for KevSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for KevSource {
    fn info(&self) -> &SourceInfo {
        &self.info
    }

    fn fetch<'a>(
        &'a self,
        http: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<FetchOutput, SourceError>> {
        Box::pin(async move {
            let mut headers = Vec::new();
            if let Some(etag) = state.get(ETAG) {
                headers.push(("If-None-Match".to_string(), etag.clone()));
            }
            if let Some(modified) = state.get(LAST_MODIFIED) {
                headers.push(("If-Modified-Since".to_string(), modified.clone()));
            }
            let response = http
                .get(HttpRequest {
                    url: CATALOG_URL.to_string(),
                    headers,
                })
                .await?;
            if response.status == 304 {
                return Ok(FetchOutput::items(Vec::new()));
            }
            if response.status != 200 {
                return Err(SourceError::Response(format!("HTTP {}", response.status)));
            }
            let catalog: Value = serde_json::from_slice(&response.body)
                .map_err(|e| SourceError::Response(format!("catalog is not JSON: {e}")))?;
            let entries = catalog
                .get("vulnerabilities")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    SourceError::Response("catalog without vulnerabilities".to_string())
                })?;
            for (key, header) in [(ETAG, "etag"), (LAST_MODIFIED, "last-modified")] {
                if let Some(value) = response.header(header) {
                    state.insert(key.to_string(), value.to_string());
                }
            }
            // Unchanged entries are recognized by the correlator from their content hash.
            let items = entries
                .iter()
                .filter_map(|entry| {
                    let id = entry.get("cveID")?.as_str()?.to_string();
                    Some(RawItem {
                        id,
                        data: entry.clone(),
                    })
                })
                .collect();
            Ok(FetchOutput::items(items))
        })
    }

    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, String> {
        normalize(&self.info.id, &item.data)
    }
}

/// Maps one catalog entry to an event; a pure function of the entry.
pub fn normalize(source: &str, entry: &Value) -> Result<NormalizedEvent, String> {
    let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::trim);
    let cve = text("cveID").ok_or("entry without cveID")?;
    let title = text("vulnerabilityName")
        .filter(|t| !t.is_empty())
        .unwrap_or(cve)
        .to_string();
    let summary = text("shortDescription")
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_SUMMARY_CHARS).collect::<String>());
    // `dateAdded` is a calendar date; it is the day the exploitation was confirmed.
    let added = text("dateAdded").and_then(|d| format!("{d}T00:00:00Z").parse::<Timestamp>().ok());
    let links: Vec<String> = text("notes")
        .unwrap_or_default()
        .split([' ', ';'])
        .filter(|t| t.starts_with("https://") || t.starts_with("http://"))
        .take(MAX_LINKS)
        .map(str::to_string)
        .collect();
    Ok(NormalizedEvent {
        schema_version: SCHEMA_VERSION.to_string(),
        source: source.to_string(),
        source_item_id: cve.to_string(),
        url: Some(format!("https://nvd.nist.gov/vuln/detail/{cve}")),
        title,
        summary,
        published_at: added,
        updated_at: added,
        kind: EventKind::Exploit,
        identifiers: Identifiers {
            cve: vec![cve.to_string()],
            ..Identifiers::default()
        },
        links,
        affected: Vec::new(),
        entities: secure_notify_model::Entities::default(),
        severity: Severity {
            exploited: true,
            ..Severity::default()
        },
        breach: None,
        incident: None,
    })
}
