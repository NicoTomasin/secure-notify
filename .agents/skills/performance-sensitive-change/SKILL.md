---
name: performance-sensitive-change
description: Keep secure-notify within its resource budgets when touching hot paths such as matching, correlation, version evaluation, parsing, dispatching, storage, WASM calls, or bulk synchronization. Identify the budget, design for indexes and streaming, benchmark before and after, and report numbers. Use when a change could affect CPU, memory, startup time, or binary size.
---

# Performance-sensitive change

**Use when:** changing code on a hot path, anything with a budget, or adding something heavy
(dependency, cache, in-memory structure).
**Not for:** speculative micro-optimizations without a measured problem.

## Read first

- [performance.md](../../../docs/architecture/performance.md) (all sections)
- [matching-and-rules.md: Matching at scale](../../../docs/architecture/matching-and-rules.md#matching-at-scale)

## Procedure

1. **Name the budget(s)** your change can affect
   ([Budgets](../../../docs/architecture/performance.md#budgets)): idle or peak memory, cold start,
   matching latency, dispatcher tick, parse time and memory, binary size.
2. **Design with the techniques**
   ([Techniques](../../../docs/architecture/performance.md#techniques)): index instead of scanning,
   stream large inputs, batch database work, bound concurrency, intern hot identifiers. Never loop
   over subscribers × stories.
3. **Measure before changing:** run the criterion benchmark for the path
   (`cargo bench -p <crate>`), adding one in `benches/` if it is missing.
4. **Measure after,** on the same machine, and compare. A regression above the threshold needs an
   explicit decision in the pull request.
5. **End to end:** for memory or throughput effects, run the binary against the synthetic workload
   (`cargo xtask workload <size>`) under limits equal to the target hardware (1 vCPU, about 1 GB).
6. **Report** before and after numbers, the machine, and the commands in the pull request.
7. Keep code readable; optimize only what the measurements show.

## Done when

- [ ] Affected budgets are named and still met, or the exception is explicitly justified.
- [ ] Benchmarks exist for the path, and before/after numbers are in the pull request.
