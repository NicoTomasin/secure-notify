---
name: source-plugin
description: Write a secure-notify source with code, native Rust or WASM. Covers scrapers that need logic, complex API clients, and bulk vulnerability database mirrors, with the fetch/normalize contract, stable source_item_id, cursors and backfill, minimal capabilities, recorded fixtures, and plugin test/record. Load wasm-plugin too when the runtime is WASM.
---

# Source plugin (code)

**Use when:** a source needs logic that the declarative runtime cannot express, or is a
high-volume core source.
**Not for:** plain feeds and simple pages ([declarative-source](../declarative-source/SKILL.md)).
For the WASM build itself, also load [wasm-plugin](../wasm-plugin/SKILL.md).

## Read first

- [plugins.md: Sources](../../../docs/architecture/plugins.md#sources),
  [Source context](../../../docs/architecture/plugins.md#source-context),
  [Plugin manifest](../../../docs/architecture/plugins.md#plugin-manifest),
  [Contribution and quality](../../../docs/architecture/plugins.md#contribution-and-quality)
- [plugin-runtimes.md: Choosing a runtime](../../../docs/architecture/plugin-runtimes.md#choosing-a-runtime)
- [data-model.md: Normalized event](../../../docs/architecture/data-model.md#normalized-event)
- [deduplication.md: Backfill](../../../docs/architecture/deduplication.md#backfill)
- [contributing.md: Acceptance criteria](../../../docs/community/contributing.md#acceptance-criteria-for-sources-and-plugins)

## Procedure

1. **Confirm the runtime:** WASM for logic; native only for high-volume core sources, agreed with a
   maintainer.
2. **Vet the source** as for a declarative source: free, terms and robots rules allow it, ethical
   boundaries respected ([vision.md](../../../docs/product/vision.md#ethical-boundaries)).
3. **Scaffold** `plugins/sources/<id>/`: `plugin.toml` (kind `source`, runtime, `api`, version,
   license, interval, `default_enabled`, `mirror`, minimal `[capabilities]`), `src/`, `fixtures/`
   with a provenance `README.md`, tests.
4. **`fetch`:** incremental, with cursors in plugin state; only the host HTTP service (conditional
   requests, rate limits, and retries come from the host); bounded pagination. The first
   synchronization is a backfill.
5. **`normalize`:** a pure function of one raw item. Stable `source_item_id`; strong identifiers
   extracted; `affected` components as purl or CPE without version, ranges OSV-style; severity only
   from what the source states; conservative entities; RFC 3339 UTC timestamps. Invalid input
   returns an error; it never panics.
6. **Mirror sources:** `mirror = true`; mirror only ecosystems present in some inventory
   ([inventory.md](../../../docs/architecture/inventory.md#inventory-stays-local)); stream bulk data
   to stay within the synchronization memory budget.
7. **Fixtures:** `plugin record <id>`; trim to the needed items; write provenance.
8. **Tests:** `normalize` over every fixture with schema validation and hand-checked expected
   fields; ID stability; malformed input (synthetic, marked)
   ([write-tests](../write-tests/SKILL.md)).

## Verify

- `plugin test <id>` with fixtures, then `--live` once; inspect events; run twice and compare IDs.
- Run the instance with the source on a scratch data directory: a story also reported by another
  source (same CVE) ends up as one story.
- For mirrors, watch peak memory during the first synchronization.

## Done when

- [ ] Manifest capabilities are minimal; the source meets the acceptance criteria.
- [ ] `normalize` is pure, IDs are stable, fixtures are recorded with provenance.
- [ ] Live and correlation checks are recorded in the pull request.
