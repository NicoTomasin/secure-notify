# Taxonomy

The taxonomy is the curated, contributable catalog of everything a subscriber can watch by name:
technologies, products, vendors, and organizations. Each entity has aliases, keywords, and
mappings to standard identifiers (purl, CPE, domains), so that structured data can be matched
exactly and free text can be tagged deterministically, without an LLM.

## Entity kinds and identifiers

> Status: `Proposed`

| Kind | ID format | Examples | Typical mappings |
|---|---|---|---|
| Technology | `tech:<slug>` | `tech:node`, `tech:postgresql`, `tech:kubernetes` | purl ecosystems and package names, CPE products |
| Product | `product:<vendor-slug>/<product-slug>` | `product:fortinet/fortios`, `product:microsoft/exchange-server` | CPE `vendor:product`, purl |
| Vendor | `vendor:<slug>` | `vendor:fortinet`, `vendor:atlassian` | CPE vendor |
| Organization | `org:<slug>` | `org:github`, `org:example-corp` | domains, status page URL, breach provider names |

- IDs are lowercase ASCII, stable, and never reused. Renames keep the old slug as an alias.
- An entity MAY declare a parent (a product's vendor, a technology's ecosystem). Watching a vendor
  includes its products.
- One real-world company can be both a vendor and an organization (e.g. it publishes products and
  runs an online service); these are separate entities linked to each other.

## Entity files

> Status: `Proposed`

One TOML file per entity in `content/taxonomy/<kind>/<slug>.toml`, so pull requests are small and
conflicts are rare:

```toml
# content/taxonomy/products/fortinet/fortios.toml
id = "product:fortinet/fortios"
name = "FortiOS"
parent = "vendor:fortinet"
aliases = ["Fortinet FortiOS", "FortiGate OS"]
keywords = ["fortios", "fortigate"]

[mappings]
cpe = ["cpe:2.3:o:fortinet:fortios"]
```

```toml
# content/taxonomy/organizations/github.toml
id = "org:github"
name = "GitHub"
aliases = ["GitHub, Inc."]
keywords = ["github"]

[mappings]
domains = ["github.com"]
statuspage = "https://www.githubstatus.com"
```

## Tagging

> Status: `Proposed`

- Structured data is mapped exactly: purl → technology/product, CPE → product/vendor, breach
  provider name or domain → organization.
- Free text (titles and summaries) is tagged by keyword matching over normalized text (multi-pattern
  matching, e.g. Aho-Corasick, with word boundaries). The result is deterministic for the same input
  and taxonomy version.
- Tagging is conservative: a false tag produces noise for every subscriber watching that entity.
  Ambiguous keywords (e.g. "Go", "Swift", "Rust", "Notion") MUST require qualifiers declared in the
  entity file (e.g. `requires_any = ["golang", "go module", "go1."]`) or be omitted.
- Tags added by the core are recorded separately from tags set by the source, so their accuracy can
  be measured.

## Contribution rules

> Status: `Proposed`

- Entity files are validated in CI against their JSON Schema (unique IDs, valid mappings, existing
  parents, no keyword collisions without qualifiers).
- Entries are factual: no marketing text, no descriptions beyond the name.
- Taxonomy changes reach instances through content bundles (see [content-registry.md](content-registry.md)).
