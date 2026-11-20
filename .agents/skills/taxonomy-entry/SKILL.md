---
name: taxonomy-entry
description: Add or fix a secure-notify taxonomy entity (technology, product, vendor, organization) or a keyword rule. One TOML file per entity with a stable ID, aliases, conservative keywords with qualifiers for ambiguous terms, and identifier mappings (CPE, purl, domains, status page) verified against authoritative sources. Use for content/taxonomy/ and content/rules/.
---

# Taxonomy entry or keyword rule

**Use when:** adding or correcting an entity, its aliases, keywords, or mappings, or a keyword rule
for tagging or severity.
**Not for:** the tagging engine itself (core code; follow
[taxonomy.md](../../../docs/architecture/taxonomy.md#tagging) with [rust-code](../rust-code/SKILL.md)).

## Read first

- [taxonomy.md](../../../docs/architecture/taxonomy.md) (all sections)
- [content-registry.md: What content contains](../../../docs/architecture/content-registry.md#what-content-contains)

## Procedure

1. **Search first:** look for the ID, name, and aliases in `content/taxonomy/`. Never create a
   duplicate or reuse a retired ID; a rename keeps the old slug as an alias.
2. **Path and ID** per [Entity kinds and identifiers](../../../docs/architecture/taxonomy.md#entity-kinds-and-identifiers):
   `content/taxonomy/<kind>/<slug>.toml` (products under their vendor), lowercase ASCII slugs, SPDX
   header, `parent` for products and child technologies.
3. **Mappings are exact and verified:** CPE vendor and product from the official CPE dictionary,
   purl types and names from the registry, domains and status page URLs from the organization
   itself. Never guess a mapping; leave it out if you cannot verify it.
4. **Keywords are conservative.** Ambiguous words (common nouns, short names, names shared with
   other things) require `requires_any` qualifiers or are omitted. A false tag notifies everyone who
   watches the entity.
5. **Factual only:** name, aliases, keywords, mappings; no descriptions or marketing.
6. **Keyword rules** in `content/rules/` follow the same caution: deterministic, specific phrases,
   no rule that would raise severity on vague wording.
7. **Validate:** `cargo run -- content verify` (unique IDs, valid mappings, existing parents, no
   keyword collisions without qualifiers).

## Verify

- Run the sources most likely to mention the entity (fixtures through `plugin test`, or a scratch
  instance with live sources) and inspect which stories get the new tag: expected ones tagged, no
  unrelated ones.
- After merge into `develop`, the entry ships with the next content bundle
  ([content-registry.md: Bundles](../../../docs/architecture/content-registry.md#bundles)).

## Done when

- [ ] No duplicate; stable ID; verified mappings; qualifiers for ambiguous keywords.
- [ ] `content verify` passes; tagging impact checked on real titles and recorded in the pull
  request.
