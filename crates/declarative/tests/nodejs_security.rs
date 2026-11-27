// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `nodejs-security` declarative source against its recorded fixture
//! (content/feeds/fixtures/nodejs-security/README.md).

#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse};
use secure_notify_core::sources::{BoxFuture, Source, SourceHttp, SourceState, normalize_items};
use secure_notify_declarative::FeedSource;
use secure_notify_model::{EventKind, SeverityLevel, load_declarative_source};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/feeds");

fn source() -> FeedSource {
    let text = std::fs::read_to_string(format!("{ROOT}/nodejs-security.toml")).unwrap();
    FeedSource::new(load_declarative_source("nodejs-security", &text).unwrap()).unwrap()
}

/// Answers with the recorded body, or `304` when the request carries the expected validator.
struct Recorded {
    requests: Mutex<Vec<HttpRequest>>,
}

impl SourceHttp for Recorded {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async move {
            let conditional = request
                .headers
                .iter()
                .any(|(n, v)| n == "If-None-Match" && v == "\"v1\"");
            self.requests.lock().unwrap().push(request);
            let body =
                std::fs::read(format!("{ROOT}/fixtures/nodejs-security/response-0.body")).unwrap();
            Ok(if conditional {
                HttpResponse {
                    status: 304,
                    headers: Vec::new(),
                    body: Vec::new(),
                }
            } else {
                HttpResponse {
                    status: 200,
                    headers: vec![("etag".to_string(), "\"v1\"".to_string())],
                    body,
                }
            })
        })
    }
}

#[tokio::test]
async fn recorded_feed_becomes_valid_events_with_the_source_defaults() {
    let source = source();
    let http = Recorded {
        requests: Mutex::new(Vec::new()),
    };
    let mut state = SourceState::new();
    let items = source.fetch(&http, &mut state).await.unwrap();
    let (events, dropped) = normalize_items(&source, &items);
    assert_eq!((events.len(), dropped), (5, 0));
    // Values read by hand from the first item of the fixture.
    let first = &events[0];
    assert_eq!(first.source, "nodejs-security");
    assert_eq!(
        first.source_item_id,
        "/blog/vulnerability/november-2026-security-releases?1794355200000"
    );
    assert_eq!(
        first.title,
        "Wednesday, November 11, 2026 Security Releases"
    );
    assert_eq!(
        first.url.as_deref(),
        Some("https://nodejs.org/en/blog/vulnerability/november-2026-security-releases")
    );
    assert_eq!(
        first.published_at.unwrap().to_string(),
        "2026-11-11T00:00:00Z"
    );
    // content/feeds/nodejs-security.toml defaults.
    assert_eq!(first.kind, EventKind::Release);
    assert_eq!(first.entities.technologies, ["tech:node"]);
    assert_eq!(first.severity.level, Some(SeverityLevel::High));
    assert_eq!(
        events[4].title,
        "Developing a minimally HashDoS resistant, yet quickly reversible integer hash for V8"
    );
}

#[tokio::test]
async fn unchanged_feeds_are_not_downloaded_again() {
    // plugins.md: conditional requests (ETag / If-None-Match).
    let source = source();
    let http = Recorded {
        requests: Mutex::new(Vec::new()),
    };
    let mut state = SourceState::new();
    assert_eq!(source.fetch(&http, &mut state).await.unwrap().len(), 5);
    assert_eq!(state.get("http.etag").map(String::as_str), Some("\"v1\""));
    assert_eq!(source.fetch(&http, &mut state).await.unwrap().len(), 0);
    assert_eq!(http.requests.lock().unwrap().len(), 2);
}
