# Database migrations

How the SQLite schema evolves. An instance upgrades by replacing its binary, so every schema change
ships as an embedded migration that runs at startup on databases of every earlier version, on small
hardware, without losing the data that deduplication and subscribers depend on. Entities are
defined in [../architecture/data-model.md](../architecture/data-model.md#persistence-entities).

## Migration model

> Status: `Decided`

- Migrations are embedded in the binary, forward-only, and applied at startup.
- Before applying migrations, the instance writes a consistent backup of the database to the data
  directory and keeps the most recent ones.
- Downgrading requires restoring a backup; this is documented in release notes when a migration is
  not reversible by older binaries (see [releases.md](releases.md#release-process)).

## Writing a migration

> Status: `Decided`

- Migrations are SQL files in `crates/storage/migrations/`, named `NNNN_<description>.sql`: a
  zero-padded sequence number with no gaps and a short snake_case description.
- One migration per logical schema change. The migration and the code that uses the new schema are
  in the same pull request.
- A migration merged into `develop` is immutable; mistakes are fixed with a new migration.
- CI rejects duplicate sequence numbers and gaps. When two pull requests use the same number, the
  one merged later fails that check before merging (see
  [git-workflow.md](git-workflow.md#merge-queue)), and its author renumbers the migration.
- Each migration runs inside a transaction and leaves the database consistent if it fails.
- A migration expressed in Rust instead of SQL is allowed only when SQL cannot express the change,
  and lives in the storage crate next to the SQL migrations.

## Changing tables in SQLite

> Status: `Decided`

- Changes that SQLite's `ALTER TABLE` does not support (changing a column type or constraint,
  dropping a column used by an index or constraint) follow SQLite's documented table rebuild
  procedure: create the new table, copy the data, drop the old table, rename the new one, recreate
  indexes and triggers.
- Foreign key enforcement is switched off around a rebuild, outside the transaction, and
  `PRAGMA foreign_key_check` MUST report no violations before it is switched back on.
- New `NOT NULL` columns get a default or are backfilled in the same migration.

## Data and privacy

> Status: `Decided`

- Migrations preserve data. Dropping data that subscribers own (configuration, inventory, watched
  identities) or that deduplication relies on (identifiers, canonical URLs, the delivery ledger; see
  [../architecture/deduplication.md](../architecture/deduplication.md#retention)) requires a doc
  change first.
- Migrations never decrypt watched identities, except in a migration whose purpose is rotating the
  encryption key.
- Tables that mirror an external database (see `mirror = true` in
  [../architecture/plugins.md](../architecture/plugins.md#plugin-manifest)) MAY be dropped and
  re-synchronized from the source instead of transformed.
- Migrations that rewrite large tables (events, stories, mirrored data) are timed against the
  synthetic workload on the target hardware (see
  [../architecture/performance.md](../architecture/performance.md#target-hardware)), and the
  timing is reported in the pull request.

## Testing migrations

> Status: `Decided`

- **Schema snapshot:** a test applies every migration to an empty database and compares the
  resulting schema with a committed snapshot, reviewed like any snapshot (see
  [testing.md](testing.md#snapshots)).
- **Upgrade:** a test loads a fixture database in the schema of the latest release (a committed SQL
  dump with representative synthetic data and no personal data), migrates it to the current schema,
  and asserts that the data is preserved and transformed as intended. The fixture is refreshed at
  each release.
- **Local verification:** the migration is applied to a fresh database and to a copy of an existing
  development database, the backup file is checked, and the affected feature is exercised (see
  [workflow.md](workflow.md#local-verification)).
