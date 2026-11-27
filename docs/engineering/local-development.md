# Local development

How to set up a machine to build, run, and verify secure-notify, and the development-only tools
that make local verification possible without production or third-party accounts. The requirement
to verify every change locally is in [workflow.md](workflow.md#local-verification).

## Toolchain

> Status: `Decided`

- Rust stable through rustup. The workspace pins the toolchain channel and components (rustfmt,
  clippy) in `rust-toolchain.toml`; the MSRV rule is in [conventions.md](conventions.md#rust).
- Cargo tools: cargo-nextest (test runner), cargo-deny (supply chain checks), cargo-insta (snapshot
  review), and cargo-mutants (mutation testing, optional locally).
- WASM plugins: the `wasm32-wasip2` target for Rust plugins; other languages use the component
  tooling documented in `sdk/<language>/`.
- Optional: the SQLite command-line shell, to inspect scratch databases.
- Core development needs no other runtime; a container engine is needed only for container work.

## Cross-platform development

> Status: `Decided`

- Linux, macOS, and Windows are supported development platforms.
- Development automation is written in Rust as `cargo xtask <task>` (the `xtask/` crate), never as
  shell scripts or tool-specific task runners, so it runs on every platform with only Cargo
  installed.
- Text files use LF line endings everywhere (enforced by `.gitattributes`); editor defaults are in
  `.editorconfig`.
- Code and tests use the standard path APIs and never assume a path separator.

## Environment

> Status: `Decided`

- Local runs read a `.env` file at the repository root, which is git-ignored. `.env.example`
  (committed) lists every environment variable the binary reads, with placeholder values and a
  comment each; it is created together with the binary crate and kept current (see
  [configuration.md](configuration.md#documenting-settings)).
- `DATA_DIR` points to a scratch directory under `.local/` (git-ignored), for example
  `.local/data`. Deleting that directory resets the local instance.
- Platform tokens in `.env` belong to a test bot created by the contributor for development. The
  token or database of an instance with real subscribers is never used for development.
- `.env` files, scratch databases, logs, and recorded data containing personal data are never
  committed. Agents MUST NOT print secret values from the environment into shared terminals, logs,
  commits, or pull request text.

## Developer commands

> Status: `Decided`

| Command | Purpose |
|---|---|
| `cargo xtask ci` | Run locally every check that CI runs on a pull request: formatting, lints, tests, cargo-deny, crate dependencies, license headers, schema, migration, docs, and skills checks. `cargo xtask <check>` runs one check; `cargo xtask` without arguments lists them. |
| `cargo xtask gen-schemas` | Regenerate the JSON Schemas in `schemas/` from the types in `crates/model` (see [../architecture/data-model.md](../architecture/data-model.md#canonical-schema)). |
| `cargo run -- run` | Run the instance with the settings in `.env`. |
| `cargo run --features dev -- dev console` | Run the instance with the [console platform](#console-platform), interactively. |
| `cargo run --features dev -- dev console --script <file>` | Run a console session from a script, print the transcript, and exit. |
| `cargo run -- migrate` | Apply pending migrations (normally done at startup). |
| `cargo run -- backup <path>` | Write a consistent backup of the database. |
| `cargo run -- plugin test <id>` | Run a plugin against its fixtures. |
| `cargo run -- plugin test <id> --live` | Run a plugin once against its real source. |
| `cargo run -- plugin record <id>` | Record new fixtures from the real source. |
| `cargo run -- content verify` | Validate content files and manifests against their schemas. |
| `cargo xtask workload <size>` | Generate the synthetic workload (see [../architecture/performance.md](../architecture/performance.md#measuring)). |

## Development-only features

> Status: `Decided`

- Development-only behavior lives behind the `dev` cargo feature. Release builds and release CI
  jobs MUST NOT enable it, and CI checks that they do not.
- With `dev`, the binary offers:
  - the [console platform](#console-platform), with event ingestion and the development clock;
  - acceptance of content bundles signed with a development key given in the environment, so the
    content updater can be tested locally (see
    [../architecture/content-registry.md](../architecture/content-registry.md)).

## Console platform

> Status: `Decided`

The console platform lets anyone, including an agent without access to Telegram or any other
platform, run the real instance and exercise its flows end to end: pairing, commands, uploads,
correlation, matching, delivery, digests, and update notices. Only the platform transport is
replaced; everything else is the real binary on a scratch database.

- `dev console` runs the full instance with the console as an additional platform. Platforms
  configured in `.env` stay enabled, so a story ingested in the console can also be delivered to a
  test bot.
- **Interactive mode** reads lines from standard input. **Script mode** (`--script <file>`) runs a
  file line by line, waits after each line until the instance is idle (no pending pipeline work and
  no notification due), prints a transcript, and exits with a non-zero status if a step failed or
  an error was logged.
- Empty lines and lines starting with `#` are skipped. Until the first `:as`, lines come from the
  user `console`.
- Lines starting with `/` are bot commands. Lines starting with `:` are console directives:

  | Directive | Effect |
  |---|---|
  | `:as <user>` | Act as a user (created on first use); following lines come from that user. |
  | `:private`, `:group <name>` | Send following lines from the user's private chat or from a group chat (created on first use). |
  | `:upload <path>` | Send a local file as an upload in the current chat. |
  | `:ingest <file>` | Ingest normalized events (canonical schema, see [../architecture/data-model.md](../architecture/data-model.md#canonical-schema)) from a JSON file as new, non-backfill events. |
  | `:advance <duration>` | Move the development clock forward (`90s`, `10m`, `1h`, `1d`). |
  | `:expect <text>` | Fail the script unless the last bot output contains the text. |

- The **development clock** replaces the injected clock in the console instance. Grace windows,
  digests, quiet hours, code expirations, and retention behave as if the advanced time had passed.
  Source schedules keep using real time, so advancing the clock never causes extra requests to
  third-party services.
- Codes the bot generates (pairing, invite, link) can be referenced in later lines as
  `{pairing_code}`, `{invite_code}`, and `{link_code}`, so scripts do not depend on random values.
- Bot output is printed with the plain-text renderer, prefixed by the destination chat. It contains
  no real personal data, so a transcript can be pasted into a pull request as verification evidence
  and the script attached for reviewers to rerun.
- The console does not exercise platform-specific rendering or transport: changes to a real
  platform plugin are still verified on that platform with a test bot (see
  [workflow.md](workflow.md#local-verification)).

Illustrative script:

```text
:as alice
/start {pairing_code}
:upload fixtures/package-lock.json
:expect components
:ingest fixtures/advisory-event.json
:advance 10m
:expect lodash
```

## Live sources during development

> Status: `Decided`

- Live fetches (`plugin test --live`, a running instance) reach real third-party services: keep
  source intervals at their defaults or longer, run live checks once rather than in loops, and never
  disable rate limiting.
- The first synchronization of each source is a backfill and does not notify (see
  [../architecture/deduplication.md](../architecture/deduplication.md#backfill)). To see
  notifications locally, ingest events through the console (`:ingest`) and advance the development
  clock past the grace window (`:advance`).
