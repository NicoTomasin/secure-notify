# Performance

Performance and low resource usage are product features: secure-notify MUST run comfortably on
small, cheap hardware so that anyone can self-host it. This doc sets the target hardware, the
budgets every change is measured against, the techniques that keep the core within them, and how
regressions are caught.

## Target hardware

> Status: `Decided`

An instance with its default configuration MUST run well on 1 vCPU and about 1 GB of total RAM,
on both arm64 and x86_64 Linux (a small VPS or a single-board computer).

## Budgets

> Status: `Proposed`

Measured on the target hardware with the default sources, Telegram enabled, and the web panel off:

| Metric | Budget |
|---|---|
| Resident memory, idle | < 100 MB |
| Resident memory, peak during bulk database synchronization | < 300 MB |
| Cold start to ready (excluding first-run synchronization) | < 2 s |
| Matching one changed story against 10,000 subscribers with 1,000,000 inventory entries | < 50 ms |
| Dispatcher tick with 10,000 subscribers | < 1 s |
| Parsing a 10 MB lock file | < 1 s, < 50 MB extra memory |
| Release binary size (default features) | < 60 MB |

Budgets are reviewed when the reference workload changes; exceeding one requires an explicit
decision in the pull request.

## Techniques

> Status: `Proposed`

- **Index, do not scan.** Inverted indexes from components, entities, and identity exposures to
  subscribers; never loops of subscribers × stories (see
  [matching-and-rules.md](matching-and-rules.md)).
- **Stream large inputs.** Lock files, SBOMs, and bulk database dumps are parsed in a streaming
  fashion; full documents are never held in memory when avoidable.
- **Batch database work.** Prepared statements, batched inserts in transactions, one writer task,
  read connection pool, WAL mode.
- **Bound concurrency.** Per-host request limits and global fetch concurrency limits; bounded
  channels between stages provide backpressure.
- **Mirror selectively.** Bulk vulnerability data is mirrored only for ecosystems present in some
  inventory (see [inventory.md](inventory.md)).
- **Cheap WASM.** Ahead-of-time compilation cached on disk and instance pooling (see
  [plugin-runtimes.md](plugin-runtimes.md)); heavy parsing offered as host functions.
- **Compact representations.** Interned component identifiers and entity IDs in hot paths.
- **Cargo features.** Platforms and native plugins behind features, so custom builds include only
  what they need.

## Measuring

> Status: `Proposed`

- Micro-benchmarks live next to the code (`benches/`, criterion) for matching, version
  evaluation, correlation, parsing, and rendering.
- A synthetic workload generator creates subscribers, inventories, and story streams of a given
  size for end-to-end measurements.
- CI runs benchmarks on pull requests that touch hot paths and on the default branch, and flags
  regressions above a threshold (default 10 %).
- Memory budgets are checked in a scheduled CI job that runs the binary against the synthetic
  workload under a memory limit equal to the target hardware.
