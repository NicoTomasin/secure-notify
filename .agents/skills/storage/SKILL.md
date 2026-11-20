---
name: storage
description: Write secure-notify's repository-layer code over SQLite. Implement core's storage traits, route writes through the single writer task, batch in transactions, rely on schema constraints for idempotency, keep the ledger and queue updates atomic, index hot lookups, and test against real SQLite. Use when adding or changing persistence code in crates/storage.
---

# Storage code

**Use when:** adding or changing queries, repository implementations, transactions, or the writer
task in `crates/storage/`.
**Not for:** schema changes alone ([database-migration](../database-migration/SKILL.md)).

## Read first

- [overview.md: Process model](../../../docs/architecture/overview.md#process-model) and
  [Cross-cutting concerns](../../../docs/architecture/overview.md#cross-cutting-concerns)
  (idempotency, time)
- [overview.md: Crate dependencies](../../../docs/architecture/overview.md#crate-dependencies)
- [performance.md: Techniques](../../../docs/architecture/performance.md#techniques)
- [data-model.md: Persistence entities](../../../docs/architecture/data-model.md#persistence-entities)

## Procedure

1. **Port first:** the operation is a method on a trait defined in `core`, using domain types. The
   storage crate implements it. No SQL or rusqlite types appear in `core`.
2. **Writes** go through the single writer task; group them in transactions with prepared
   statements; **reads** use the read pool. All database calls run off the async executor.
3. **Idempotency through constraints:** rely on unique keys (`(source, source_item_id)`, ledger and
   queue keys) with upserts, instead of check-then-insert sequences that race.
4. **Atomic delivery bookkeeping:** the ledger row and the queue item's status change commit in the
   same transaction
   ([deduplication.md](../../../docs/architecture/deduplication.md#double-check-before-dispatch)).
5. **Query plans:** every hot lookup has an index; check with `EXPLAIN QUERY PLAN`. No loops that
   query once per subscriber or per story.
6. **Time and data:** timestamps stored in UTC; watched identities only through their encrypted and
   hashed columns ([exposure-monitoring.md](../../../docs/architecture/exposure-monitoring.md#storage));
   row contents never logged.
7. **Tests** use real SQLite (in memory or a temporary file) with the real migrations applied, never
   a mocked database ([testing.md](../../../docs/engineering/testing.md#forbidden-test-patterns)).
   Cover constraint violations and concurrent-looking sequences (same item twice, retry after
   failure).

## Verify

- Run the instance on a scratch data directory and exercise the feature; inspect the tables
  directly.
- For hot paths, compare benchmarks before and after
  ([performance-sensitive-change](../performance-sensitive-change/SKILL.md)).

## Done when

- [ ] `core` sees only its trait; SQL stays in `storage`.
- [ ] Writes are batched through the writer task; invariants are enforced by the schema.
- [ ] Tests run on real SQLite; hot queries are indexed.
