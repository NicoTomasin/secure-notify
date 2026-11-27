// SPDX-License-Identifier: MIT OR Apache-2.0

//! Field rules of docs/architecture/data-model.md#normalized-event.

use secure_notify_model::{EventKind, RangeEvent, RangeType, ValidationError, parse_event};
use serde_json::{Value, json};

/// The documented example event, filled with the values of a real advisory:
/// <https://github.com/advisories/GHSA-35jh-r3h4-6jhm> (command injection in lodash, CVE-2021-23337).
fn lodash_advisory() -> Value {
    json!({
        "schema_version": "1",
        "source": "osv",
        "source_item_id": "GHSA-35jh-r3h4-6jhm",
        "url": "https://osv.dev/vulnerability/GHSA-35jh-r3h4-6jhm",
        "title": "Command Injection in lodash",
        "summary": "lodash versions prior to 4.17.21 are vulnerable to Command Injection via the template function.",
        "published_at": "2021-02-15T21:05:01Z",
        "updated_at": "2024-03-22T21:34:41Z",
        "kind": "vulnerability",
        "identifiers": { "cve": ["CVE-2021-23337"], "ghsa": ["GHSA-35jh-r3h4-6jhm"], "osv": [], "other": [] },
        "links": ["https://github.com/lodash/lodash/commit/3469357cff396a26c363f8c1b5a91dde28ba4b1c"],
        "affected": [
            {
                "component": { "purl": "pkg:npm/lodash" },
                "ranges": [{ "type": "semver", "events": [{ "introduced": "0" }, { "fixed": "4.17.21" }] }],
                "fixed": ["4.17.21"]
            }
        ],
        "entities": { "technologies": ["tech:node"], "products": [], "vendors": [], "organizations": [] },
        "severity": {
            "level": "high",
            "cvss": { "version": "3.1", "score": 7.2, "vector": "CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H" },
            "epss": null,
            "exploited": false,
            "exploit_available": false
        }
    })
}

fn parse(value: &Value) -> Result<secure_notify_model::NormalizedEvent, ValidationError> {
    parse_event(value.to_string().as_bytes())
}

/// Applies `edit` to the example and returns the fields reported as invalid.
fn rejected_fields(edit: impl FnOnce(&mut Value)) -> Vec<String> {
    let mut value = lodash_advisory();
    edit(&mut value);
    match parse(&value) {
        Ok(_) => Vec::new(),
        Err(ValidationError::Rules(errors)) => errors.into_iter().map(|e| e.field).collect(),
        Err(ValidationError::Shape(message)) => vec![format!("shape: {message}")],
    }
}

#[test]
fn documented_event_shape_is_accepted() {
    let event = parse(&lodash_advisory()).expect("the documented shape must validate");
    assert_eq!(event.kind, EventKind::Vulnerability);
    assert_eq!(event.identifiers.cve, ["CVE-2021-23337"]);
    let range = &event.affected[0].ranges[0];
    assert_eq!(range.kind, RangeType::Semver);
    assert_eq!(
        range.events,
        [
            RangeEvent::Introduced("0".to_string()),
            RangeEvent::Fixed("4.17.21".to_string())
        ]
    );
}

#[test]
fn each_broken_field_rule_is_rejected_with_its_field() {
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit, &str); 21] = [
        (
            "unsupported schema version",
            |v| v["schema_version"] = json!("2"),
            "schema_version",
        ),
        (
            "empty source_item_id",
            |v| v["source_item_id"] = json!(""),
            "source_item_id",
        ),
        ("empty title", |v| v["title"] = json!("  "), "title"),
        (
            "non-http url",
            |v| v["url"] = json!("javascript:alert(1)"),
            "url",
        ),
        (
            "non-http link",
            |v| v["links"] = json!(["ftp://example.com/x"]),
            "links[0]",
        ),
        (
            "CVE with a two-digit year",
            |v| v["identifiers"]["cve"] = json!(["CVE-21-23337"]),
            "identifiers.cve[0]",
        ),
        (
            "CVE with a three-digit number",
            |v| v["identifiers"]["cve"] = json!(["CVE-2021-233"]),
            "identifiers.cve[0]",
        ),
        (
            "purl without type",
            |v| v["affected"][0]["component"]["purl"] = json!("pkg:/lodash"),
            "affected[0].component.purl",
        ),
        (
            "purl without name",
            |v| v["affected"][0]["component"]["purl"] = json!("pkg:npm/"),
            "affected[0].component.purl",
        ),
        (
            "malformed GHSA",
            |v| v["identifiers"]["ghsa"] = json!(["GHSA-35jh"]),
            "identifiers.ghsa[0]",
        ),
        (
            "CVE in osv",
            |v| v["identifiers"]["osv"] = json!(["CVE-2021-23337"]),
            "identifiers.osv[0]",
        ),
        (
            "other without namespace",
            |v| v["identifiers"]["other"] = json!(["RUSTSEC-2021-0001"]),
            "identifiers.other[0]",
        ),
        (
            "both purl and cpe",
            |v| {
                v["affected"][0]["component"]["cpe"] =
                    json!("cpe:2.3:a:lodash:lodash:*:*:*:*:*:node.js:*:*")
            },
            "affected[0].component",
        ),
        (
            "neither purl nor cpe",
            |v| v["affected"][0]["component"] = json!({}),
            "affected[0].component",
        ),
        (
            "purl with a version",
            |v| v["affected"][0]["component"]["purl"] = json!("pkg:npm/lodash@4.17.20"),
            "affected[0].component.purl",
        ),
        (
            "cpe with a version",
            |v| {
                v["affected"][0]["component"] =
                    json!({ "cpe": "cpe:2.3:a:lodash:lodash:4.17.20:*:*:*:*:node.js:*:*" })
            },
            "affected[0].component.cpe",
        ),
        (
            "technology without prefix",
            |v| v["entities"]["technologies"] = json!(["node"]),
            "entities.technologies[0]",
        ),
        (
            "cvss score above 10",
            |v| v["severity"]["cvss"]["score"] = json!(10.5),
            "severity.cvss.score",
        ),
        (
            "epss score above 1",
            |v| v["severity"]["epss"] = json!({ "score": 1.5, "percentile": 0.5 }),
            "severity.epss.score",
        ),
        (
            "breach block on a vulnerability",
            |v| v["breach"] = json!({ "data_classes": ["email"] }),
            "breach",
        ),
        (
            "incident block on a vulnerability",
            |v| v["incident"] = json!({ "status": "investigating" }),
            "incident",
        ),
    ];
    for (case, edit, field) in cases {
        assert_eq!(rejected_fields(edit), [field], "case: {case}");
    }
}

#[test]
fn valid_variants_of_the_field_rules_are_accepted() {
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit); 5] = [
        // The purl spec percent-encodes the `@` of npm scopes.
        ("scoped npm purl", |v| {
            v["affected"][0]["component"]["purl"] = json!("pkg:npm/%40babel/traverse")
        }),
        (
            "cpe with version *",
            |v| {
                v["affected"][0]["component"] =
                    json!({ "cpe": "cpe:2.3:a:lodash:lodash:*:*:*:*:*:node.js:*:*" })
            },
        ),
        ("namespaced other identifier", |v| {
            v["identifiers"]["other"] = json!(["rustsec:RUSTSEC-2021-0001"])
        }),
        ("malicious package OSV ID", |v| {
            v["identifiers"]["osv"] = json!(["MAL-2025-1234"])
        }),
        ("incident block on an outage", |v| {
            v["kind"] = json!("outage");
            v["incident"] = json!({ "status": "resolved", "impact": "minor" });
        }),
    ];
    for (case, edit) in cases {
        assert_eq!(rejected_fields(edit), Vec::<String>::new(), "case: {case}");
    }
}

#[test]
fn unknown_kind_is_rejected() {
    let fields = rejected_fields(|v| v["kind"] = json!("rumor"));
    assert_eq!(fields.len(), 1);
    assert!(fields[0].starts_with("shape:"), "{fields:?}");
}

#[test]
fn timestamps_with_an_offset_are_stored_in_utc() {
    let mut value = lodash_advisory();
    value["published_at"] = json!("2021-02-15T23:05:01+02:00");
    let event = parse(&value).unwrap();
    assert_eq!(
        event.published_at.unwrap().to_string(),
        "2021-02-15T21:05:01Z"
    );
}

#[test]
fn fields_added_by_a_later_additive_change_are_ignored() {
    // Additive changes keep `schema_version` (data-model.md#canonical-schema), so an event from a
    // producer that knows a newer optional field still validates.
    let mut value = lodash_advisory();
    value["future_optional_field"] = json!({ "anything": true });
    assert!(parse(&value).is_ok_and(|e| e.source_item_id == "GHSA-35jh-r3h4-6jhm"));
}
