---
name: inventory
description: Change secure-notify's inventory core. Covers the upload flow (parser selection, limits, in-memory parsing), flat anonymous component entries merged as a union, manual entries, stale-entry removal, derived technology watches, the initial scan and its ledger records, and the privacy guarantees. Use for inventory code in crates/core and upload handling; file formats belong to parser-plugin.
---

# Inventory and uploads

**Use when:** changing how uploads are processed, how entries are stored, merged, or removed, or
how the initial scan works.
**Not for:** supporting a file format ([parser-plugin](../parser-plugin/SKILL.md)).

## Read first

- [inventory.md](../../../docs/architecture/inventory.md) (all sections)
- [deduplication.md: Backfill](../../../docs/architecture/deduplication.md#backfill) (initial scans
  use backfilled stories)
- [subscribers.md: Permissions in groups](../../../docs/architecture/subscribers.md#permissions-in-groups)

## Procedure

1. **Privacy invariants are `Decided`:** files are never stored, in whole or in part; entries are
   flat component + version pairs with no grouping, project names, file names, or hashes; no
   follow-up questions after an upload; no logging of contents or entries. Load
   [security-sensitive-change](../security-sensitive-change/SKILL.md).
2. **Before parsing:** select the parser by filename, enforce size and complexity limits, parse off
   the async executor with a timeout.
3. **Merge as a union:** add new pairs, refresh `last_seen_at` on existing ones, remove nothing
   automatically.
4. **Derived technologies** become watches with origin `derived`.
5. **Initial scan:** check new entries once against local data, including backfilled stories; send a
   single report; record its stories in the ledger so they are not sent again as new.
6. **Inventory stays local:** no entry is ever sent to a third party; matching indexes are updated
   on every change ([matching](../matching/SKILL.md)).
7. **Removal paths:** the inline "I no longer use this version" action, `/inventory remove`, and
   `/inventory clear`.
8. **Tests:** two uploads produce the union; the initial scan writes ledger rows and a later
   ingestion of the same story does not notify; after an upload, no file bytes exist in the database
   or data directory; group uploads by non-managers are refused.

## Verify

In `dev console`: `:upload <path>` with a real lock file, check the summary and the single
initial scan report; upload a second file and check the union in `/inventory`; remove an entry;
inspect `inventory_entries` and confirm the file content is nowhere in the data directory or logs.

## Done when

- [ ] Every privacy invariant holds and is covered by a test.
- [ ] Union merge, initial scan, and ledger recording verified locally.
