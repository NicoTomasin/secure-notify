// SPDX-License-Identifier: AGPL-3.0-or-later

//! The KEV source against its recorded fixture (fixtures/README.md). Expected values were read by
//! hand from the fixture.

#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse};
use secure_notify_core::sources::{
    BoxFuture, Download, FetchOutput, RawItem, Source, SourceHttp, SourceState, normalize_items,
};
use secure_notify_model::EventKind;
use secure_notify_source_cisa_kev::{CATALOG_URL, KevSource};

const BODY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/response-0.body");

/// Serves the recorded catalog, or `304` when the request carries the expected validator.
struct Recorded {
    requests: Mutex<Vec<HttpRequest>>,
}

impl SourceHttp for Recorded {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async move {
            assert_eq!(request.url, CATALOG_URL);
            let conditional = request
                .headers
                .iter()
                .any(|(n, v)| n == "If-None-Match" && v == "\"kev-1\"");
            self.requests.lock().unwrap().push(request);
            Ok(if conditional {
                HttpResponse {
                    status: 304,
                    headers: Vec::new(),
                    body: Vec::new(),
                }
            } else {
                HttpResponse {
                    status: 200,
                    headers: vec![("etag".to_string(), "\"kev-1\"".to_string())],
                    body: std::fs::read(BODY).unwrap(),
                }
            })
        })
    }

    fn download(&self, _: HttpRequest, _: u64) -> BoxFuture<'_, Result<Download, HttpError>> {
        Box::pin(async { Err(HttpError::Refused("not used".to_string())) })
    }
}

fn items(output: FetchOutput) -> Vec<RawItem> {
    output.batches.flat_map(Result::unwrap).collect()
}

#[tokio::test]
async fn catalog_entries_become_exploited_events_for_their_cve() {
    let source = KevSource::new();
    let http = Recorded {
        requests: Mutex::new(Vec::new()),
    };
    let mut state = SourceState::new();
    let items = items(source.fetch(&http, &mut state).await.unwrap());
    let (events, dropped) = normalize_items(&source, &items);
    assert_eq!((events.len(), dropped), (3, 0));
    let node = events
        .iter()
        .find(|e| e.source_item_id == "CVE-2021-21315")
        .unwrap();
    assert_eq!(node.kind, EventKind::Exploit);
    assert_eq!(node.identifiers.cve, ["CVE-2021-21315"]);
    assert!(node.severity.exploited);
    assert_eq!(
        node.title,
        "System Information Library for Node.JS Command Injection"
    );
    assert_eq!(
        node.published_at.unwrap().to_string(),
        "2022-01-18T00:00:00Z"
    );
    assert_eq!(
        node.url.as_deref(),
        Some("https://nvd.nist.gov/vuln/detail/CVE-2021-21315")
    );
    // URLs listed in the entry's notes become links.
    let geoserver = events
        .iter()
        .find(|e| e.source_item_id == "CVE-2026-120705")
        .unwrap();
    assert!(
        geoserver.links.contains(
            &"https://github.com/geoserver/geoserver/security/advisories/GHSA-39mw-5chv-jh47"
                .to_string()
        )
    );
    assert!(geoserver.links.iter().all(|l| l.starts_with("https://")));
}

#[tokio::test]
async fn an_unchanged_catalog_is_not_downloaded_again() {
    let source = KevSource::new();
    let http = Recorded {
        requests: Mutex::new(Vec::new()),
    };
    let mut state = SourceState::new();
    assert_eq!(
        items(source.fetch(&http, &mut state).await.unwrap()).len(),
        3
    );
    assert_eq!(
        items(source.fetch(&http, &mut state).await.unwrap()).len(),
        0
    );
}
