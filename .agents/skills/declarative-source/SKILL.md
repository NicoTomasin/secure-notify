---
name: declarative-source
description: Add or fix a no-code source in secure-notify as one TOML file in content/feeds/. Covers RSS/Atom/JSON Feed, JSON APIs, static HTML scraping with CSS selectors, status pages, and canonical feed subscriptions, including vetting the source's terms and robots rules, stable IDs, conservative defaults, and live checks. Use for feeds and simple scraping; use source-plugin when logic is needed.
---

# Declarative source

**Use when:** the source is a feed, a JSON endpoint mappable with JSONPath, a static HTML page
mappable with CSS selectors, a status page, or another instance's canonical feed.
**Not for:** sources needing logic, login flows, per-item detail requests, or complex pagination
([source-plugin](../source-plugin/SKILL.md)). Pages rendered by JavaScript are not supported by
any runtime; look for the API the page calls instead.

## Read first

- [plugin-runtimes.md: Choosing a runtime](../../../docs/architecture/plugin-runtimes.md#choosing-a-runtime)
  and [Declarative runtime](../../../docs/architecture/plugin-runtimes.md#declarative-runtime)
- [plugins.md: Sources](../../../docs/architecture/plugins.md#sources)
- [contributing.md: Acceptance criteria](../../../docs/community/contributing.md#acceptance-criteria-for-sources-and-plugins)
- [sources-catalog.md: Free access](../../../docs/product/sources-catalog.md#free-access) and
  [Catalog rules](../../../docs/product/sources-catalog.md#catalog-rules)
- [scope.md: Event kinds](../../../docs/product/scope.md#event-kinds)

## Procedure

1. **Vet the source:** public and free; terms allow automated access; robots rules allow the path;
   not a leak site or underground forum
   ([vision.md](../../../docs/product/vision.md#ethical-boundaries)). Keep links to the terms and
   robots rules for the pull request.
2. **Choose the `type`:** prefer structured data (JSON API, feed) over `html`.
3. **Choose the `id`:** kebab-case, unique, stable. It becomes `source` on every event and is what
   subscriber rules match on, so it never changes after release.
4. **Write `content/feeds/<id>.toml`** with the SPDX header: `name`, `type`, `url`, a reasonable
   `interval`, `default_enabled` per
   [Default enablement](../../../docs/product/sources-catalog.md#default-enablement), and
   `[defaults]` with a conservative `kind`, `entities`, and `severity`.
5. **Mappings (`html`, `json-api`):** `source_item_id` from a real identifier or the item's
   canonical URL, never from position or date alone; `url` absolute; dates with an explicit format;
   pagination with a page limit.
6. **Entities:** only existing taxonomy IDs; add missing ones first
   ([taxonomy-entry](../taxonomy-entry/SKILL.md)).
7. **API keys:** `secret = "<NAME>"`; the source stays disabled until the operator provides it.
   Mention the variable in the pull request.
8. **Fixture:** for `html` and `json-api`, record one with `plugin record <id>` and write its
   provenance ([testing.md: Fixtures](../../../docs/engineering/testing.md#fixtures)).
9. **Validate:** `cargo run -- content verify`.

## Verify

- `cargo run -- plugin test <id>` (fixture, when present), then `--live` once.
- Inspect the events: titles clean, URLs absolute, dates in UTC, identifiers (CVE, GHSA, ...)
  extracted, kind and entities as intended.
- Run the live test a second time: the same items have the same `source_item_id`.
- After merge into `develop`, the source reaches instances with the next content bundle if the
  latest release supports every feature it uses, otherwise with the next release
  ([content-registry.md: Bundles](../../../docs/architecture/content-registry.md#bundles)).

## Done when

- [ ] The source passes the acceptance criteria, with evidence of terms and robots rules.
- [ ] `content verify` passes; IDs are stable across runs; defaults are conservative.
- [ ] Live output was inspected and summarized in the pull request.
