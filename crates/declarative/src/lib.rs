// SPDX-License-Identifier: AGPL-3.0-or-later

//! Declarative source engine for secure-notify (docs/architecture/plugin-runtimes.md#declarative-runtime).
//!
//! A declarative source is one TOML file in `content/feeds/`; no code. The engine fetches through
//! the shared HTTP client, uses conditional requests, and turns each feed entry into a raw item
//! that `normalize` maps to an event with the source's defaults and the identifiers found in its
//! text and link.

use jiff::Timestamp;
use secure_notify_core::identifiers;
use secure_notify_core::ports::HttpRequest;
use secure_notify_core::sources::{
    BoxFuture, RawItem, Source, SourceError, SourceHttp, SourceInfo, SourceState,
};
use secure_notify_core::time::parse_duration;
use secure_notify_model::{
    DeclarativeSource, Entities, EventKind, Identifiers, NormalizedEvent, SCHEMA_VERSION, Severity,
};
use serde_json::json;

/// State keys for conditional requests.
const ETAG: &str = "http.etag";
const LAST_MODIFIED: &str = "http.last_modified";
/// Longest summary kept from a feed entry.
const MAX_SUMMARY_CHARS: usize = 1000;

/// A declarative feed source (`rss` or `atom`).
pub struct FeedSource {
    definition: DeclarativeSource,
    info: SourceInfo,
}

impl FeedSource {
    /// Builds a source from a validated definition.
    pub fn new(definition: DeclarativeSource) -> Result<Self, String> {
        let interval = parse_duration(&definition.interval)?;
        let host = url_host(&definition.url)
            .ok_or_else(|| format!("{}: url has no host", definition.id))?;
        let info = SourceInfo {
            id: definition.id.clone(),
            name: definition.name.clone(),
            interval,
            mirror: false,
            default_enabled: definition.default_enabled,
            structured: false,
            hosts: vec![host],
        };
        Ok(Self { definition, info })
    }
}

fn url_host(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// Plain text from feed HTML: tags removed, common entities decoded, whitespace collapsed.
fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn http_link(link: Option<&str>) -> Option<String> {
    let link = link?.trim();
    let lower = link.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://")).then(|| link.to_string())
}

impl Source for FeedSource {
    fn info(&self) -> &SourceInfo {
        &self.info
    }

    fn fetch<'a>(
        &'a self,
        http: &'a dyn SourceHttp,
        state: &'a mut SourceState,
    ) -> BoxFuture<'a, Result<Vec<RawItem>, SourceError>> {
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
                    url: self.definition.url.clone(),
                    headers,
                })
                .await?;
            if response.status == 304 {
                return Ok(Vec::new());
            }
            if response.status != 200 {
                return Err(SourceError::Response(format!("HTTP {}", response.status)));
            }
            let feed = feed_rs::parser::parse(response.body.as_slice())
                .map_err(|e| SourceError::Response(format!("not a feed: {e}")))?;
            for (key, header) in [(ETAG, "etag"), (LAST_MODIFIED, "last-modified")] {
                if let Some(value) = response.header(header) {
                    state.insert(key.to_string(), value.to_string());
                }
            }
            let items = feed
                .entries
                .into_iter()
                .map(|entry| {
                    let link = entry.links.first().map(|l| l.href.clone());
                    let summary = entry
                        .summary
                        .map(|s| s.content)
                        .or_else(|| entry.content.and_then(|c| c.body));
                    RawItem {
                        id: entry.id.clone(),
                        data: json!({
                            "id": entry.id,
                            "title": entry.title.map(|t| t.content),
                            "link": link,
                            "summary": summary,
                            "published": entry.published.map(|d| d.to_rfc3339()),
                            "updated": entry.updated.map(|d| d.to_rfc3339()),
                        }),
                    }
                })
                .collect();
            Ok(items)
        })
    }

    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, String> {
        let data = &item.data;
        let text = |key: &str| data.get(key).and_then(serde_json::Value::as_str);
        let id = text("id")
            .filter(|s| !s.trim().is_empty())
            .ok_or("entry without id")?;
        let title = plain_text(text("title").ok_or("entry without title")?);
        if title.is_empty() {
            return Err("entry with an empty title".to_string());
        }
        let url = http_link(text("link"));
        let summary = text("summary")
            .map(plain_text)
            .filter(|s| !s.is_empty())
            .map(|s| s.chars().take(MAX_SUMMARY_CHARS).collect::<String>());
        let time = |key: &str| text(key).and_then(|t| t.parse::<Timestamp>().ok());
        let searchable = format!(
            "{title} {} {}",
            summary.as_deref().unwrap_or_default(),
            url.as_deref().unwrap_or_default()
        );
        let (cve, ghsa) = identifiers::extract(&searchable);
        let defaults = &self.definition.defaults;
        let mut entities = Entities::default();
        for entity in &defaults.entities {
            let list = match entity.split(':').next() {
                Some("tech") => &mut entities.technologies,
                Some("product") => &mut entities.products,
                Some("vendor") => &mut entities.vendors,
                _ => &mut entities.organizations,
            };
            list.push(entity.clone());
        }
        Ok(NormalizedEvent {
            schema_version: SCHEMA_VERSION.to_string(),
            source: self.info.id.clone(),
            source_item_id: id.trim().to_string(),
            links: url.clone().into_iter().collect(),
            url,
            title,
            summary,
            published_at: time("published"),
            updated_at: time("updated"),
            kind: defaults.kind.unwrap_or(EventKind::News),
            identifiers: Identifiers {
                cve,
                ghsa,
                osv: Vec::new(),
                other: Vec::new(),
            },
            affected: Vec::new(),
            entities,
            severity: Severity {
                level: defaults.severity,
                ..Severity::default()
            },
            breach: None,
            incident: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_html_becomes_plain_text() {
        assert_eq!(
            plain_text("<p>Fixes &amp; <b>CVE-2024-1234</b>&nbsp;now</p>"),
            "Fixes & CVE-2024-1234 now"
        );
    }

    #[test]
    fn only_http_links_are_kept() {
        assert_eq!(http_link(Some("javascript:alert(1)")), None);
        assert_eq!(
            http_link(Some(" https://nodejs.org/x ")),
            Some("https://nodejs.org/x".to_string())
        );
    }
}
