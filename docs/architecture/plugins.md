# Plugins

Everything that connects secure-notify to the outside world is a plugin with a small, typed
contract: **sources** (where information comes from), **parsers** (which files the bot
understands), **enrichers** (what extra data stories get), **exposure providers** (how watched
identities are checked), and **platform plugins** (where messages go and commands come from).
Plugin authors write only the domain logic; the core provides scheduling, HTTP, state,
validation, sandboxing, and error handling. How plugins are executed (native, declarative, WASM)
is described in [plugin-runtimes.md](plugin-runtimes.md).

## Requirements

> Status: `Decided`

- Adding a new source MUST be easy.
- The community MUST be able to contribute plugins (news, feeds, APIs, scraping) via pull requests.
- Plugins MUST be writable in any language that compiles to a WebAssembly component.

## Plugin kinds

> Status: `Proposed`

| Kind | Input | Output | Runtimes |
|---|---|---|---|
| Source | Schedule tick, its own state | Raw items, then normalized events | native, declarative, WASM |
| Parser | One uploaded file (bytes, filename) | Components and technologies | native, WASM |
| Enricher | One changed story | Additional story fields | native, WASM |
| Exposure provider | One verified watched identity | Breach identifiers it appears in | native, WASM |
| Platform plugin | Rendered messages; platform events | Deliveries; commands, uploads, button actions | native |

## Sources

> Status: `Proposed`

- A source has two steps: `fetch` returns raw items; `normalize` turns one raw item into one
  normalized event (see [data-model.md](data-model.md)).
- `normalize` MUST be a pure function of one raw item.
- `normalize` output is validated against the event schema; invalid events are dropped and logged,
  and never crash the pipeline.
- `source_item_id` MUST be stable across fetches of the same item.
- Sources keep cursors in their persistent state and fetch incrementally.
- A source's first synchronization is a backfill (see [deduplication.md](deduplication.md)).
- Sources MUST respect the target site's terms and robots rules, use reasonable intervals, and
  follow the ethical boundaries in [../product/vision.md](../product/vision.md).
- Most feed sources need no code: see the declarative runtime in
  [plugin-runtimes.md](plugin-runtimes.md).

Illustrative native contract:

```rust
pub trait Source: Send + Sync {
    fn id(&self) -> &str;
    async fn fetch(&self, ctx: &SourceContext) -> Result<Vec<RawItem>, SourceError>;
    fn normalize(&self, item: &RawItem) -> Result<NormalizedEvent, NormalizeError>;
}
```

### Source context

> Status: `Proposed`

Every runtime receives the same host services:

| Service | Provides |
|---|---|
| HTTP | Fetch with per-host rate limiting, conditional requests (ETag / If-Modified-Since), retries with backoff, timeouts, response size limits, SSRF protection, and a generic User-Agent `secure-notify/<version> (+<canonical repository URL>)` (see [../product/licensing.md](../product/licensing.md)). Never includes subscriber or operator data. |
| State | Persistent per-plugin key/value store for cursors. |
| Parse helpers | RSS/Atom/JSON Feed parsing, HTML selection, JSON path queries, executed natively by the host. |
| Log | Structured logger scoped to the plugin. |
| Clock | Current time (injectable in tests). |
| Secrets | Values of the secrets the plugin declared (e.g. an API key), configured by the operator. |

## Parsers

> Status: `Proposed`

- Parsers receive file bytes and the filename only. They MUST NOT execute file content, perform
  network calls, access the filesystem, or keep state.
- Parsers MUST use safe parsing and respect size and complexity limits enforced by the core.
- Output is limited to components (purl or CPE, optional version) and technologies. Parsers MUST
  NOT emit project names, file paths, or other identifying metadata (see "Privacy" in
  [inventory.md](inventory.md)).
- Manifests with version ranges (`^1.2.0`) produce components without an exact version; lock
  files and SBOMs produce exact versions, including transitive dependencies.
- Detection is by filename first; an optional `detect(filename, content)` MAY disambiguate (e.g.
  CycloneDX vs SPDX JSON).

## Enrichers

> Status: `Proposed`

- Enrichers run after correlation, on stories that are new or changed.
- They MAY only add or update the fields they declare (e.g. `severity.epss`,
  `severity.exploit_available`, entity tags); they MUST NOT delete data or change identifiers.
- Enrichers that need bulk data (e.g. the daily EPSS file) fetch it on their own schedule and
  answer from local state, never with one request per story.

## Exposure providers

> Status: `Proposed`

- Exposure providers receive one verified watched identity and return the breach identifiers it
  appears in. They handle personal data and therefore MUST declare exactly one provider host and
  MUST NOT have persistent state.
- Rules for identities are defined in [exposure-monitoring.md](exposure-monitoring.md).

## Platform plugins

> Status: `Proposed`

- A platform plugin MUST implement delivery and MAY implement interaction (commands, uploads,
  buttons). Its contract and the conversation layer are described in [platforms.md](platforms.md).
- The core MUST NOT contain platform-specific code outside platform plugins.

## Plugin manifest

> Status: `Proposed`

Every native and WASM plugin has a `plugin.toml`. Declarative sources are their own manifest (see
[plugin-runtimes.md](plugin-runtimes.md)).

```toml
id = "cisa-kev"
name = "CISA Known Exploited Vulnerabilities"
kind = "source"                              # source | parser | enricher | exposure-provider | platform
runtime = "wasm"                             # native | wasm
api = "secure-notify:plugin/source@1.0.0"    # WIT world the plugin implements
version = "0.1.0"
license = "AGPL-3.0-or-later"
interval = "1h"                              # sources and scheduled enrichers only
default_enabled = true
mirror = false                               # true for sources that mirror a whole database (exempt from retention)

[capabilities]
hosts = ["www.cisa.gov"]                     # the only hosts the plugin may contact
secrets = []                                 # named secrets the operator must provide
state = true
```

- The core enforces capabilities; a plugin cannot reach a host or secret it did not declare.
- A plugin that requires a secret stays disabled until the operator provides it.
- An update that expands a plugin's capabilities is not activated until an admin approves it.

## Contribution and quality

> Status: `Proposed`

- Every code plugin ships **fixtures** (real, recorded responses or files) and tests that run
  `normalize`, `parse`, or `enrich` against them and validate the output schema. CI runs them for
  every runtime and language.
- Declarative sources are validated in CI against their schema and fetched in a scheduled CI job to
  detect breakage.
- `secure-notify plugin test <id>` runs a plugin locally against fixtures or live data;
  `secure-notify plugin record <id>` records new fixtures.
- Built-in plugins are reviewed via pull request before merge. The contribution workflow is in
  [../community/contributing.md](../community/contributing.md).

## External plugins

> Status: `Proposed`

- Plugins outside the repository are distributed as WASM components with their manifest, or
  through additional content registries (see [content-registry.md](content-registry.md)).
- External plugins MUST be enabled explicitly in the instance config; nothing is loaded
  automatically.
- Running third-party plugins is the operator's decision; docs MUST state clearly that capability
  sandboxing reduces but does not remove that risk.
