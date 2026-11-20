# Engineering conventions

How code in this repository is written and checked. These conventions apply to the Rust workspace
and to plugins in every language; plugin-specific requirements are in
[../architecture/plugins.md](../architecture/plugins.md). Testing rules are in
[testing.md](testing.md); branching, commits, and pull requests in [git-workflow.md](git-workflow.md);
the lifecycle of a change in [workflow.md](workflow.md).

## Rust

> Status: `Proposed`

- Stable toolchain, edition 2024. The minimum supported Rust version is the latest stable minus two
  minor releases, declared in the workspace `Cargo.toml` and checked in CI.
- Lints are configured once in the workspace (`[workspace.lints]`): `unsafe_code = "forbid"` (an
  exception requires a documented justification in the crate), clippy warnings denied in CI, a
  curated set of pedantic lints.
- Formatting with rustfmt defaults.
- Errors: typed errors with thiserror in libraries; context is added at the binary boundary. No
  `unwrap`/`expect` outside tests, except for invariants proven locally and explained in a comment.
- Async code uses tokio. Blocking or CPU-heavy work (SQLite, large parsing, WASM calls) runs off
  the async executor.
- Time, randomness, and I/O reach domain logic through traits, so that logic is deterministic and
  testable with fakes (see [../architecture/overview.md](../architecture/overview.md#crate-dependencies)).
- Public items have doc comments. Comments explain why, not what.
- Crates are named `secure-notify-<name>`; plugin crates `secure-notify-<kind>-<id>`.

## Dependencies

> Status: `Proposed`

- Prefer well-maintained, widely used crates; disable default features that are not needed.
- Dependency versions are declared once, in `[workspace.dependencies]`; member crates refer to them
  with `workspace = true`.
- TLS uses rustls; OpenSSL is not linked.
- Every new dependency is justified in the pull request and must pass cargo-deny (see
  [../product/licensing.md](../product/licensing.md) and [../architecture/security.md](../architecture/security.md)).

## Logging

> Status: `Proposed`

- Use `tracing` with structured fields; JSON output in production.
- What MUST NOT be logged is defined in [../architecture/security.md](../architecture/security.md).

## Continuous integration

> Status: `Proposed`

CI runs on GitHub Actions:

- formatting, lints, tests (all workspace crates and plugins), MSRV build;
- workspace dependency graph against the crate dependency table (see
  [../architecture/overview.md](../architecture/overview.md#crate-dependencies));
- cargo-deny (licenses, advisories, bans, sources);
- schema validation for content, manifests, and generated JSON Schemas (generated files must be up
  to date);
- WASM plugin builds with each language's toolchain, then their fixture tests;
- link check over `docs/`, `.agents/`, and the root Markdown files, and a check that the skills
  index lists exactly the skill folders that exist (see [workflow.md](workflow.md#agent-skills));
- migration checks: no duplicate or missing sequence numbers, schema snapshot and upgrade tests
  (see [migrations.md](migrations.md));
- a check that no release build enables the `dev` feature (see
  [local-development.md](local-development.md#development-only-features));
- benchmarks on relevant pull requests; scheduled jobs for live source checks, memory budgets, and
  mutation testing (see [testing.md](testing.md#mutation-check));
- release builds for every supported target (see [releases.md](releases.md)).

The pull request checks also run in the merge queue, on the result of merging into the latest
`develop` (see [git-workflow.md](git-workflow.md#merge-queue)). Every pull request check can be run
locally with `cargo xtask ci` (see [local-development.md](local-development.md#developer-commands)).
