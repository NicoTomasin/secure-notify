// SPDX-License-Identifier: AGPL-3.0-or-later

//! The OSV npm mirror against real records (fixtures/README.md). Expected values were read by
//! hand from the records.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse};
use secure_notify_core::sources::{
    BoxFuture, Download, FetchOutput, RawItem, Source, SourceHttp, SourceState, normalize_items,
};
use secure_notify_model::{EventKind, RangeEvent, SeverityLevel};
use secure_notify_source_osv::OsvSource;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
const BASE: &str = "https://osv-vulnerabilities.storage.googleapis.com/npm";

/// Serves the fixtures by URL and records the requested URLs.
struct Fixtures {
    files: BTreeMap<String, String>,
    requested: Mutex<Vec<String>>,
}

impl Fixtures {
    fn new() -> Self {
        let mut files = BTreeMap::new();
        files.insert(format!("{BASE}/all.zip"), "all.zip".to_string());
        files.insert(
            format!("{BASE}/modified_id.csv"),
            "modified_id.csv".to_string(),
        );
        for id in [
            "MAL-2026-20723",
            "GHSA-35jh-r3h4-6jhm",
            "GHSA-3966-f6p6-2qr9",
        ] {
            files.insert(format!("{BASE}/{id}.json"), format!("entries/{id}.json"));
        }
        Self {
            files,
            requested: Mutex::new(Vec::new()),
        }
    }

    fn path(&self, url: &str) -> Result<String, HttpError> {
        self.requested.lock().unwrap().push(url.to_string());
        self.files
            .get(url)
            .map(|f| format!("{FIXTURES}/{f}"))
            .ok_or_else(|| HttpError::Refused(format!("no fixture for {url}")))
    }
}

impl SourceHttp for Fixtures {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(async move {
            let body = std::fs::read(self.path(&request.url)?).unwrap();
            Ok(HttpResponse {
                status: 200,
                headers: Vec::new(),
                body,
            })
        })
    }

    fn download(&self, request: HttpRequest, _: u64) -> BoxFuture<'_, Result<Download, HttpError>> {
        Box::pin(async move {
            let file = std::fs::File::open(self.path(&request.url)?).unwrap();
            let len = file.metadata().unwrap().len();
            let headers = vec![(
                "last-modified".to_string(),
                "Fri, 27 Nov 2026 20:33:35 GMT".to_string(),
            )];
            Ok(Download {
                status: 200,
                headers,
                file,
                len,
            })
        })
    }
}

fn items(output: FetchOutput) -> Vec<RawItem> {
    output.batches.flat_map(Result::unwrap).collect()
}

#[tokio::test]
async fn the_bulk_synchronization_yields_every_npm_record_but_withdrawn_ones() {
    let source = OsvSource::new();
    let http = Fixtures::new();
    let mut state = SourceState::new();
    let output = source.fetch(&http, &mut state).await.unwrap();
    assert!(output.withdrawn.is_empty());
    let items = items(output);
    let (events, dropped) = normalize_items(&source, &items);
    assert_eq!(dropped, 0);
    let mut ids: Vec<&str> = events.iter().map(|e| e.source_item_id.as_str()).collect();
    ids.sort_unstable();
    // Six records in the archive; GHSA-3966-f6p6-2qr9 is withdrawn.
    assert_eq!(
        ids,
        [
            "GHSA-35jh-r3h4-6jhm",
            "GHSA-4j66-8f4r-3pjx",
            "GHSA-f82v-jwr5-mffw",
            "GHSA-xvch-5gv4-984h",
            "MAL-2026-20723"
        ]
    );
    // The cursor is the archive's Last-Modified time.
    assert_eq!(
        state.get("cursor").map(String::as_str),
        Some("2026-11-27T20:33:35Z")
    );
}

#[tokio::test]
async fn records_are_normalized_with_aliases_npm_packages_and_severity() {
    let source = OsvSource::new();
    let http = Fixtures::new();
    let events = {
        let mut state = SourceState::new();
        let items = items(source.fetch(&http, &mut state).await.unwrap());
        normalize_items(&source, &items).0
    };
    let lodash = events
        .iter()
        .find(|e| e.source_item_id == "GHSA-35jh-r3h4-6jhm")
        .unwrap();
    assert_eq!(lodash.title, "Command Injection in lodash");
    assert_eq!(lodash.identifiers.cve, ["CVE-2021-23337"]);
    assert_eq!(lodash.identifiers.ghsa, ["GHSA-35jh-r3h4-6jhm"]);
    assert_eq!(lodash.kind, EventKind::Vulnerability);
    assert_eq!(lodash.severity.level, Some(SeverityLevel::High));
    // Four npm packages; the RubyGems entry of the record is not part of the npm mirror.
    let purls: Vec<&str> = lodash
        .affected
        .iter()
        .filter_map(|a| a.component.purl.as_deref())
        .collect();
    assert_eq!(
        purls,
        [
            "pkg:npm/lodash",
            "pkg:npm/lodash-es",
            "pkg:npm/lodash.template",
            "pkg:npm/lodash-template"
        ]
    );
    assert_eq!(lodash.affected[0].fixed, ["4.17.21"]);
    assert_eq!(
        lodash.affected[2].ranges[0].events,
        [
            RangeEvent::Introduced("0".to_string()),
            RangeEvent::LastAffected("4.5.0".to_string())
        ]
    );
    let malware = events
        .iter()
        .find(|e| e.source_item_id == "MAL-2026-20723")
        .unwrap();
    assert_eq!(malware.kind, EventKind::MaliciousPackage);
    assert_eq!(malware.identifiers.osv, ["MAL-2026-20723"]);
    assert_eq!(malware.identifiers.ghsa, ["GHSA-3vvx-6qr4-p6hp"]);
    assert_eq!(
        malware.severity.level, None,
        "level is derived later: malicious packages are critical"
    );
    let summary = malware.summary.as_deref().unwrap();
    assert!(
        summary.starts_with("Any computer that has this package installed"),
        "{summary}"
    );
    assert!(
        !summary.contains("Per source details"),
        "boilerplate must be removed"
    );
}

#[tokio::test]
async fn later_runs_fetch_only_records_modified_since_the_cursor_and_report_withdrawals() {
    let source = OsvSource::new();
    let http = Fixtures::new();
    let mut state = SourceState::new();
    // Between GHSA-f82v-jwr5-mffw (03:50:23) and the withdrawn GHSA-3966-f6p6-2qr9 (03:50:32).
    state.insert("cursor".to_string(), "2026-09-10T03:50:30Z".to_string());
    let output = source.fetch(&http, &mut state).await.unwrap();
    assert_eq!(output.withdrawn, ["GHSA-3966-f6p6-2qr9"]);
    let ids: Vec<String> = items(output).into_iter().map(|i| i.id).collect();
    assert_eq!(ids, ["MAL-2026-20723", "GHSA-35jh-r3h4-6jhm"]);
    let requested = http.requested.lock().unwrap().clone();
    assert!(requested.iter().all(|u| !u.ends_with("all.zip")));
    assert_eq!(
        state.get("cursor").map(String::as_str),
        Some("2026-11-27T19:45:17.26030799Z")
    );
}

#[test]
fn github_severities_map_to_the_five_levels() {
    // matching-and-rules.md#severity-normalization: an explicit level is mapped to the five
    // levels. The record is the real lodash advisory with its severity replaced.
    let text =
        std::fs::read_to_string(format!("{FIXTURES}/entries/GHSA-35jh-r3h4-6jhm.json")).unwrap();
    let cases = [
        ("CRITICAL", Some(SeverityLevel::Critical)),
        ("HIGH", Some(SeverityLevel::High)),
        ("MODERATE", Some(SeverityLevel::Medium)),
        ("LOW", Some(SeverityLevel::Low)),
        ("SOMETHING", None),
    ];
    for (github, expected) in cases {
        let mut record: serde_json::Value = serde_json::from_str(&text).unwrap();
        record["database_specific"]["severity"] = serde_json::json!(github);
        let event = secure_notify_source_osv::normalize("osv", &record).unwrap();
        assert_eq!(event.severity.level, expected, "{github}");
    }
}
