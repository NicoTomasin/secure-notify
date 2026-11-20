---
name: rust-code
description: Write or restructure Rust code in the secure-notify workspace. Place code in the crate that owns the concern, respect the ports-and-adapters dependency direction, handle errors, keep blocking work off the async executor, inject time and I/O, log safely, and create new crates correctly. Use for any Rust change, together with the skill for the area.
---

# Rust code

**Use when:** writing or restructuring Rust code anywhere in the workspace, or creating a crate.
**Not for:** adding dependencies ([add-dependency](../add-dependency/SKILL.md)) or schema changes
([database-migration](../database-migration/SKILL.md)).

## Read first

- [overview.md: Repository layout](../../../docs/architecture/overview.md#repository-layout) and
  [Crate dependencies](../../../docs/architecture/overview.md#crate-dependencies)
- [conventions.md: Rust](../../../docs/engineering/conventions.md#rust) and
  [Logging](../../../docs/engineering/conventions.md#logging)
- [licensing.md: Source files and packages](../../../docs/product/licensing.md#source-files-and-packages)

## Procedure

1. **Place the code** in the crate that owns the concern (repository layout). If it would need a
   workspace dependency the table does not allow, define a trait (port) in `core` and implement it
   in the adapter crate instead.
2. **New crate:** named `secure-notify-<name>` (plugins `secure-notify-<kind>-<id>`); add it to the
   workspace members; `[lints] workspace = true`; `license` field; SPDX header on every file; a
   crate-level doc comment. Add it to the layout and dependency table in
   [overview.md](../../../docs/architecture/overview.md) ([change-docs](../change-docs/SKILL.md)).
3. **Errors:** one thiserror enum per library crate with variants callers can act on; context is
   added at the binary boundary. No `unwrap`/`expect` outside tests unless the invariant is proven
   locally and explained in a comment.
4. **Async and blocking:** tokio for I/O; SQLite, large parsing, and WASM calls run off the executor
   (blocking tasks or the writer task). Never hold a database connection or a lock across `.await`.
   Channels between stages are bounded.
5. **Determinism:** time, randomness, HTTP, and storage reach domain logic through traits; domain
   functions are pure where possible, so tests use fakes at the ports.
6. **Logging:** `tracing` with structured fields; never the data
   [security.md: Logging](../../../docs/architecture/security.md#logging) forbids; endpoints only by
   internal ID.
7. **User-facing text** goes through the i18n layer ([i18n](../i18n/SKILL.md)).
8. **Documentation:** public items have doc comments; comments explain why, not what.
9. **No `unsafe`** (forbidden by workspace lints); no platform-specific code outside platform
   plugins ([plugins.md](../../../docs/architecture/plugins.md#platform-plugins)).

## Verify

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, or all of
  them through `cargo xtask ci`.
- `cargo tree -p secure-notify-core` shows no adapter crate.

## Done when

- [ ] The code lives in the owning crate and respects the dependency table.
- [ ] Errors, async boundaries, logging, and i18n follow the conventions.
- [ ] Lints and formatting pass with no new warnings.
