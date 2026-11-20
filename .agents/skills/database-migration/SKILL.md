---
name: database-migration
description: Change secure-notify's SQLite schema safely. Write a forward-only embedded migration with the next sequence number, use the SQLite table rebuild procedure when needed, preserve subscriber data and the deduplication ledger, test the schema snapshot and the upgrade from the last release, and verify on a copy of a real development database. Use for any new or changed table, column, index, or constraint.
---

# Database migration

**Use when:** anything under `crates/storage/migrations/` changes, or code needs a new table,
column, index, or constraint.
**Not for:** queries and repository code without schema changes ([storage](../storage/SKILL.md)).

## Read first

- [migrations.md](../../../docs/engineering/migrations.md) (all sections)
- [data-model.md: Persistence entities](../../../docs/architecture/data-model.md#persistence-entities)
- [deduplication.md: Retention](../../../docs/architecture/deduplication.md#retention): what must
  survive
- [releases.md: Versioning](../../../docs/engineering/releases.md#versioning): compatibility promise

## Procedure

1. **Docs first:** if an entity or its key fields change, update the entity table in
   [data-model.md](../../../docs/architecture/data-model.md#persistence-entities)
   ([change-docs](../change-docs/SKILL.md)).
2. **Next number:** list `crates/storage/migrations/` and create
   `NNNN_<snake_case_description>.sql` with the next number.
3. **Write the SQL:** one logical change. For changes `ALTER TABLE` cannot do, follow the table
   rebuild procedure in
   [Changing tables in SQLite](../../../docs/engineering/migrations.md#changing-tables-in-sqlite),
   including `PRAGMA foreign_key_check`. New `NOT NULL` columns get a default or a backfill.
4. **Protect data:** never drop subscriber-owned data or deduplication records without a doc
   decision; never decrypt watched identities
   ([Data and privacy](../../../docs/engineering/migrations.md#data-and-privacy)). Mirror tables may
   be dropped and re-synchronized instead.
5. **Update the repository code** that uses the schema in the same pull request
   ([storage](../storage/SKILL.md)).
6. **Tests** ([Testing migrations](../../../docs/engineering/migrations.md#testing-migrations)):
   update the schema snapshot and review its diff line by line; make the upgrade test pass from the
   last release's fixture; add assertions for every row you transform.
7. **Large tables:** time the migration against the synthetic workload and report it in the pull
   request.
8. **Collision:** if another pull request took the same number first (the duplicate-number check
   fails, often in the merge queue), renumber yours, then rerun the tests. Never edit a migration that is already on `develop`; fix forward with a new one.

## Verify

- Fresh database: start the instance and check the schema.
- Copy of your development database: `cargo run -- migrate`; a backup file appears in the data
  directory; the instance starts; the affected feature works.
- Say in the pull request whether an older binary can still open the migrated database (release
  notes need it).

## Done when

- [ ] One immutable, correctly numbered migration plus the code that uses it.
- [ ] Snapshot reviewed, upgrade test passing, transformed data asserted.
- [ ] Local verification on a fresh and an existing database recorded.
