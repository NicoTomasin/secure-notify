# Data model

Every source, regardless of origin or runtime, produces the same normalized event shape. Events
are grouped into stories, and everything subscriber-specific (endpoints, watches, inventory,
identities, rules, queue, delivery ledger) hangs off subscribers. Normalization into a common
model is a core requirement; each section's status label says how settled its shape is.

## Normalization requirement

> Status: `Decided`

All incoming information MUST be normalized into a common model so it can be compared across
sources, deduplicated, and matched against subscribers' watches, inventory, and identities.

## Canonical schema

> Status: `Decided`

- The normalized event is defined once, as Rust types in `crates/model`. The JSON Schema in
  `schemas/` is generated from those types and is the language-neutral contract.
- Field names are `snake_case`. Timestamps are RFC 3339 in UTC.
- WASM plugins, declarative sources, and the canonical feed all exchange events as JSON validated
  against this schema; there is no second definition of the event in WIT.
- The schema carries a `schema_version`. Additive changes keep the version; breaking changes bump
  it and follow [../engineering/releases.md](../engineering/releases.md).

## Normalized event

> Status: `Decided`

Illustrative JSON (a vulnerability event):

```json
{
  "schema_version": "1",
  "source": "osv",
  "source_item_id": "GHSA-xxxx-xxxx-xxxx",
  "url": "https://osv.dev/vulnerability/GHSA-xxxx-xxxx-xxxx",
  "title": "Prototype pollution in example-lib",
  "summary": "…",
  "published_at": "2026-11-20T10:00:00Z",
  "updated_at": "2026-11-20T12:00:00Z",
  "kind": "vulnerability",
  "identifiers": { "cve": ["CVE-2026-1234"], "ghsa": ["GHSA-xxxx-xxxx-xxxx"], "osv": [], "other": [] },
  "links": ["https://github.com/example/example-lib/security/advisories/GHSA-xxxx-xxxx-xxxx"],
  "affected": [
    {
      "component": { "purl": "pkg:npm/example-lib" },
      "ranges": [{ "type": "semver", "events": [{ "introduced": "0" }, { "fixed": "4.17.21" }] }],
      "fixed": ["4.17.21"]
    }
  ],
  "entities": { "technologies": ["tech:node"], "products": [], "vendors": [], "organizations": [] },
  "severity": {
    "level": "high",
    "cvss": { "version": "3.1", "score": 7.5, "vector": "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:H/A:N" },
    "epss": null,
    "exploited": false,
    "exploit_available": false
  }
}
```

Field rules:

| Field | Rule |
|---|---|
| `source`, `source_item_id` | Unique together; `source_item_id` MUST be stable across fetches. |
| `kind` | One of the event kinds defined in [../product/scope.md](../product/scope.md). |
| `identifiers` | Strong IDs only. `other` holds namespaced IDs such as `rustsec:RUSTSEC-2026-0001`, `breach:<provider>:<name>`, `statuspage:<host>:<incident-id>`. |
| `affected[].component` | Exactly one of `purl` (without version) or `cpe` (CPE 2.3 with the version part as `*`). |
| `affected[].ranges` | OSV-style events (`introduced`, `fixed`, `last_affected`, `limit`). Range `type`: `semver`, `ecosystem` (ordering defined by the purl type), `cpe` (generic version ordering), or `git` (not evaluated against inventories). |
| `entities` | Taxonomy IDs (see [taxonomy.md](taxonomy.md)). Sources MAY set them; the core adds more through deterministic tagging. |
| `severity.level` | Optional at ingestion; derived when absent (see [matching-and-rules.md](matching-and-rules.md)). |
| `severity.epss` | `{ "score": 0.0–1.0, "percentile": 0.0–1.0 }`, usually set by an enricher. |

Kind-specific blocks, present only when relevant:

```json
"breach": {
  "organization": "org:example-corp",
  "disclosed_at": "2026-11-18T00:00:00Z",
  "occurred_at": "2026-10-01T00:00:00Z",
  "records": 2000000,
  "data_classes": ["email", "password-hash", "name", "phone"]
},
"incident": {
  "organization": "org:github",
  "status": "investigating",
  "impact": "major",
  "started_at": "2026-11-20T09:12:00Z",
  "resolved_at": null
}
```

- `data_classes` values: `email`, `username`, `name`, `phone`, `address`, `date-of-birth`,
  `ip-address`, `location`, `password`, `password-hash`, `security-qa`, `auth-token`,
  `private-messages`, `financial`, `payment-card`, `government-id`, `health`, `biometric`, `other`.
- `incident.status`: `investigating`, `identified`, `monitoring`, `resolved`, `scheduled`.
  `incident.impact`: `none`, `minor`, `major`, `critical`. `incident` is used by `attack` and
  `outage` events.

## Story

> Status: `Proposed`

A story aggregates its events:

- **Kind:** the most specific kind among its events, within one family (families are defined in
  [../product/scope.md](../product/scope.md)). Precedence, vulnerability family:
  `malicious-package` > `vulnerability` > `bug` > `release` > `exploit` > `news`. Incident family:
  `breach` > `attack` > `outage` > `news`. `exploit` events joining a vulnerability story set
  `exploit_available` or `exploited` on it.
- **Severity:** the highest level among its events; `exploited` and `exploit_available` are true if
  any event says so; EPSS is the latest value.
- **Identifiers, affected, entities:** the union of its events'.
- **Title:** from the highest-priority source (structured databases before feeds before news).
- **Related stories:** cross-family references (e.g. a breach report naming the CVE that was
  exploited) are stored as links between stories, never as merges.

## Persistence entities

> Status: `Proposed`

Conceptual entities; table names and columns are finalized with the schema.

| Entity | Key fields | Notes |
|---|---|---|
| `subscribers` | id, kind (`person`/`team`), role (`admin`/`member`, persons only), timezone, language, min_severity?, active, created_at | See [subscribers.md](subscribers.md); `min_severity` in [matching-and-rules.md](matching-and-rules.md#minimum-severity). |
| `endpoints` | id, subscriber_id, platform, address, kind (`private`/`group`/`channel`/`email`/`webhook`), label, primary, verified, active | One subscriber, many endpoints. |
| `link_codes` | code, subscriber_id, expires_at | Linking additional endpoints. |
| `invites` | code, created_by, kind (`person`/`team`), expires_at, used_by | Single-use. |
| `registration_requests` | id, platform, address, status, created_at | Open registration with manual approval. |
| `watches` | subscriber_id, entity_id, origin (`manual`/`derived`) | Watched entities. |
| `inventory_entries` | subscriber_id, component, version?, last_seen_at (unique subscriber+component+version) | Flat and anonymous. See [inventory.md](inventory.md). |
| `watched_identities` | id, subscriber_id, kind (`email`/`domain`), value_encrypted, value_hash, verified_at, last_checked_at | See [exposure-monitoring.md](exposure-monitoring.md). |
| `identity_exposures` | identity_id, story_id, first_seen_at | Which breaches a watched identity appears in. |
| `source_state` | source_id, key, value | Cursors, backfill status, health (last success, last error). |
| `events` | id, source, source_item_id (unique together), story_id, payload, content_hash, fetched_at, backfill | |
| `stories` | id, title, kind, severity_level, cvss, epss, exploited, exploit_available, fix_available, first_seen_at, updated_at, eligible_at, backfill, compacted | `eligible_at` implements the grace window; `compacted` marks a minimal deduplication record (see "Retention" in [deduplication.md](deduplication.md)). |
| `story_identifiers` | story_id, kind, value (unique kind+value) | Identifier correlation index. |
| `story_affected` | story_id, component, ranges, fixed | Union of events' `affected`. |
| `story_entities` | story_id, entity_id | |
| `story_relations` | story_id, related_story_id | Cross-family references. |
| `rules` | id, subscriber_id, position, match, action, min_severity | See [matching-and-rules.md](matching-and-rules.md). |
| `mutes` | subscriber_id, scope (`story`/`entity`/`component`/`component-version`), value, until? | |
| `delivery_prefs` | subscriber_id, per-severity mode and routing, quiet hours, update-notice triggers | See [delivery.md](delivery.md). |
| `notification_queue` | subscriber_id, story_id, kind (`new`/`update`), due_at, status | |
| `deliveries` | subscriber_id, story_id, kind, snapshot, sent_at | The delivery ledger. |
| `delivery_attempts` | delivery or queue item, endpoint_id, attempt, status, error, at | Per-endpoint retries. |
| `web_sessions` | id, subscriber_id, created_at, last_seen_at, expires_at | See [web-panel.md](web-panel.md). |
| `content_state` | registry, installed_version, previous_version, applied_at | See [content-registry.md](content-registry.md). |
| `plugin_state` | plugin_id, version, digest, enabled, approved_capabilities | |
