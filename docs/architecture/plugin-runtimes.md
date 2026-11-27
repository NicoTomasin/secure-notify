# Plugin runtimes

A plugin runs in one of three runtimes: **native** (Rust, compiled into the binary),
**declarative** (a TOML file interpreted by the core, no code), or **WASM** (a WebAssembly
component written in any language, sandboxed inside the process). The runtime changes how a
plugin is built and isolated, not what it does: every runtime implements the same contracts
described in [plugins.md](plugins.md).

## Supported runtimes

> Status: `Decided`

- Plugins run only as native Rust code, declarative definitions, or WASM components.
- Arbitrary executables (subprocess plugins) and a push ingestion API are not supported.

## Choosing a runtime

> Status: `Proposed`

Systems outside the plugin model connect by exposing the canonical feed (see
[feed-protocol.md](feed-protocol.md)), which the instance pulls like any other source.

| Runtime | Use it for | Isolation | Who can write it |
|---|---|---|---|
| Declarative | Feeds, simple JSON APIs, simple static HTML pages, status pages, canonical feeds | Interpreted by the core; no code | Anyone; no programming needed |
| WASM | Anything that needs logic: complex APIs, scrapers, parsers, enrichers | Sandboxed: memory, CPU, and capability limits | Any language with WebAssembly component tooling |
| Native | High-volume core sources (bulk vulnerability databases), platform plugins, performance-critical parsers | None beyond Rust's memory safety; reviewed as core code | Rust contributors, in-repo only |

## Declarative runtime

> Status: `Proposed`

A declarative source is one TOML file in `content/feeds/`. Supported types:

| `type` | Input | Notes |
|---|---|---|
| `rss`, `atom`, `json-feed` | A feed URL | Entries become events; identifiers (CVE, GHSA, ...) and links are extracted from content. |
| `json-api` | A JSON endpoint | Field mapping with JSONPath; page or cursor pagination with a page limit. |
| `html` | A static HTML page | Field mapping with CSS selectors; no JavaScript execution. |
| `statuspage` | A status page base URL | Common status page JSON APIs and incident feeds; produces `outage` events. |
| `secure-notify-feed` | A canonical feed URL | Federation and external systems (see [feed-protocol.md](feed-protocol.md)). |

```toml
# content/feeds/nodejs-security.toml
id = "nodejs-security"
name = "Node.js security releases"
type = "rss"
url = "https://nodejs.org/en/feed/vulnerability.xml"
interval = "30m"
default_enabled = true

[defaults]
kind = "release"
entities = ["tech:node"]
severity = "high"
```

```toml
# content/feeds/example-vendor-advisories.toml
id = "example-vendor-advisories"
name = "Example vendor advisories"
type = "html"
url = "https://vendor.example/security/advisories"
interval = "2h"

[mapping]
items = "table.advisories tbody tr"
source_item_id = { selector = "td.id" }
title = { selector = "td.title" }
url = { selector = "td.title a", attribute = "href" }
published_at = { selector = "td.date", format = "%Y-%m-%d" }

[defaults]
kind = "vulnerability"
entities = ["vendor:example"]
```

- The core applies `defaults`, extracts identifiers and links from content, and tags entities
  through the taxonomy (see [taxonomy.md](taxonomy.md)).
- Implemented types: `rss` and `atom`. Entries become events with the source's `defaults`; `source_item_id`
  is the entry ID (`guid` in RSS), the link is the event URL, and conditional requests (ETag,
  Last-Modified) avoid downloading unchanged feeds.
- Recorded fixtures of a declarative source live in `content/feeds/fixtures/<id>/`: the bodies
  written by `plugin record <id>`, an `index.json` mapping each URL to its body, and a provenance
  `README.md`.
- Declarative sources that need an API key declare it (`secret = "EXAMPLE_API_KEY"`) and stay
  disabled until the operator provides it.

## WASM runtime

> Status: `Decided` (WASM components are the polyglot mechanism) / `Proposed` (details)

- Plugins are WebAssembly components implementing one WIT world per plugin kind, published in
  `sdk/wit` (e.g. `secure-notify:plugin/source@1.0.0`, `.../parser`, `.../enricher`,
  `.../exposure-provider`). WIT worlds define functions and host imports; events and other domain
  data cross the boundary as JSON validated against the canonical schema (see
  [data-model.md](data-model.md)).
- Any language with component model tooling can be used (for example Rust, Go, Python,
  JavaScript/TypeScript, C#, C/C++). The project maintains guest SDKs in `sdk/<language>/`,
  starting with Rust.
- Imports are host-mediated only: the source context services (see [plugins.md](plugins.md)),
  restricted to the capabilities in the manifest. No filesystem, sockets, environment variables,
  or subprocesses.
- Parser components get no network, state, or secret imports at all.
- Resource limits per call, configurable by the operator: memory (default 64 MiB), CPU through
  fuel or epoch interruption, wall-clock timeout (default 60 s for `fetch`, 5 s for `parse`, 1 s
  per `normalize` or `enrich` item), maximum output size and item count.
- A trap or limit violation fails that call only; repeated failures disable the plugin and notify
  admins (see [../operations/self-hosting.md](../operations/self-hosting.md)).
- Components are compiled ahead of time on first load and cached in the data directory;
  instances are pooled for repeated calls (see [performance.md](performance.md)).

Illustrative WIT:

```wit
package secure-notify:plugin@1.0.0;

world source {
  import host-http;
  import host-state;
  import host-parse;
  import host-log;
  import host-clock;
  import host-secrets;

  /// Returns raw items as opaque byte strings.
  export fetch: func() -> result<list<list<u8>>, plugin-error>;
  /// Returns one normalized event as canonical JSON.
  export normalize: func(item: list<u8>) -> result<string, plugin-error>;
}
```

## Native runtime

> Status: `Proposed`

- Native plugins are Rust crates in `plugins/<kind>/<id>/`, compiled into the binary behind cargo
  features so builds can include only what they need.
- Native is reserved for in-repo plugins where the sandbox cost matters or long-lived connections
  are needed: bulk vulnerability databases, platform plugins, and parsers for very large files.
- Native plugins follow the same contracts, manifest, and capability declarations; the core
  enforces host and secret capabilities through the shared HTTP client and secret store.

## Platform plugins and WASM

> Status: `Proposed`

Platform plugins are native only. They hold long-lived connections (gateways, WebSockets, long
polling) that the WASM runtime does not yet support well. WASM platform plugins are listed under
"Later" in [../roadmap.md](../roadmap.md).
