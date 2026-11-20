---
name: enricher-plugin
description: Write or change a secure-notify enricher that adds data to new or changed stories after correlation, such as EPSS scores, exploit availability, or entity tags. Enrichers answer from locally synchronized bulk data, only add the fields they declare, and must be accurate because their flags trigger update notices. Use for anything under plugins/enrichers/.
---

# Enricher plugin

**Use when:** adding or changing an enricher.
**Not for:** deterministic keyword tagging rules in content
([taxonomy-entry](../taxonomy-entry/SKILL.md)). For a WASM enricher, also load
[wasm-plugin](../wasm-plugin/SKILL.md).

## Read first

- [plugins.md: Enrichers](../../../docs/architecture/plugins.md#enrichers)
- [data-model.md: Story](../../../docs/architecture/data-model.md#story)
- [matching-and-rules.md: Severity normalization](../../../docs/architecture/matching-and-rules.md#severity-normalization)
  (EPSS does not change the level)
- [deduplication.md: Update notices](../../../docs/architecture/deduplication.md#update-notices)

## Procedure

1. **Declare the fields** the enricher adds or updates. It never deletes data or changes
   identifiers.
2. **Bulk data on its own schedule:** download the dataset (for example the daily EPSS file) into
   plugin state through the host HTTP service, then answer each story locally. Never one request per
   story.
3. **Idempotent:** enriching the same story twice yields the same fields.
4. **Accuracy over coverage:** `exploited` and `exploit_available` trigger update notices for every
   subscriber who received the story; set them only from authoritative data.
5. **Capabilities:** only the hosts the dataset needs; secrets only if the provider requires a key.
6. **Fixtures:** a recorded excerpt of the real dataset with provenance.
7. **Tests:** given a story and the fixture, the expected fields, checked by hand against the
   dataset; a story absent from the dataset is left unchanged; malformed dataset rows are skipped
   without failing the whole run.

## Verify

- Run `dev console` with the enricher enabled on a scratch data directory; `:ingest` events whose
  identifiers appear in the dataset; check the story fields.
- Confirm that an update notice appears only when a field that triggers notices actually changes.

## Done when

- [ ] Declared fields only; local answers from bulk data; idempotent.
- [ ] Fixture-based tests with hand-checked values; update-notice behavior verified locally.
