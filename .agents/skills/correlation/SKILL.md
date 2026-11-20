---
name: correlation
description: Change how secure-notify correlates events into stories. Covers identifier and canonical URL signals, weak organization-time and text-similarity signals, kind families, story merges with ledger merging, backfill flags, the grace window, and retention compaction. Conservative and deterministic. Use for correlator and retention code in crates/core or anything deciding whether two reports describe the same happening.
---

# Correlation and deduplication

**Use when:** changing how events attach to stories, how stories merge, backfill handling, the
grace window, or retention and compaction.
**Not for:** the delivery double check and update notices ([delivery](../delivery/SKILL.md)).

## Read first

- [deduplication.md](../../../docs/architecture/deduplication.md): sections
  [Events vs stories](../../../docs/architecture/deduplication.md#events-vs-stories),
  [Kind families](../../../docs/architecture/deduplication.md#kind-families),
  [Correlation signals](../../../docs/architecture/deduplication.md#correlation-signals),
  [Conservative merging](../../../docs/architecture/deduplication.md#conservative-merging),
  [Story merges](../../../docs/architecture/deduplication.md#story-merges),
  [Backfill](../../../docs/architecture/deduplication.md#backfill),
  [Retention](../../../docs/architecture/deduplication.md#retention),
  [Grace window](../../../docs/architecture/deduplication.md#grace-window)
- [data-model.md: Story](../../../docs/architecture/data-model.md#story)

## Procedure

1. **Name the signal or rule** you are changing; if behavior changes, update the doc first
   ([change-docs](../change-docs/SKILL.md)).
2. **Conservative by design:** when uncertain, create a new story. Weak-signal thresholds are
   configurable, with conservative defaults.
3. **Deterministic:** the same events in the same taxonomy version give the same stories. No LLM,
   no randomness, no dependence on processing order where avoidable.
4. **Families:** never merge across the vulnerability and incident families; link related stories
   instead.
5. **Merges:** aliases join the story's identifiers; the delivery ledger is merged so nobody is
   notified twice for the merged story.
6. **Backfill and compaction:** backfill flags propagate to stories; a compacted story that receives
   a new event is rebuilt and keeps its identity.
7. **Performance:** identifier and URL lookups are indexed (`story_identifiers`); similarity is
   bounded by the time window.
8. **Tests:** real duplicates from fixtures (the same CVE from a database, a vendor advisory, and a
   news item) become one story; near misses (similar titles, different CVEs) stay separate; a merge
   of two delivered stories triggers no new notification; URL canonicalization cases from the doc.

## Verify

- Scratch instance with two or more overlapping sources (backfill): inspect `stories` and
  `story_identifiers` for the expected grouping.
- In `dev console`, `:ingest` a later report of an already delivered story and `:advance` past the
  grace window: no new notification, unless it carries a material change.

## Done when

- [ ] Docs describe the signal; behavior is deterministic and conservative.
- [ ] Merge, near-miss, and backfill cases are tested on real data.
- [ ] Local run confirms grouping and no duplicate notification.
