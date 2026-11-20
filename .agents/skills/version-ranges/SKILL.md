---
name: version-ranges
description: Implement or fix version parsing, ordering, and range evaluation for an ecosystem in secure-notify's crates/versions (semver for npm and Cargo, PEP 440, Maven, Go modules, RubyGems, NuGet, Debian, RPM, Alpine, generic CPE ordering), following the official specification, with property tests and test vectors from real advisories. Use whenever exact-version matching is involved.
---

# Version ranges

**Use when:** adding an ecosystem, fixing a comparison or range bug, or changing how OSV range
events are evaluated.
**Not for:** deciding relevance levels ([matching](../matching/SKILL.md)).

## Read first

- [matching-and-rules.md: Version evaluation](../../../docs/architecture/matching-and-rules.md#version-evaluation)
- [data-model.md: Normalized event](../../../docs/architecture/data-model.md#normalized-event)
  (range field rules)

## Procedure

1. **Follow the official specification** of the ecosystem and link it in the module's doc comment.
   Never approximate one ecosystem with another's rules (PEP 440 is not semver; Debian epochs and
   tildes have their own ordering).
2. **Range events** per OSV: `introduced`, `fixed`, `last_affected`, `limit`. `git` ranges are not
   evaluated.
3. **Unknown means unknown:** an unparsable version or an unsupported ecosystem returns an explicit
   "cannot evaluate" result, which leads to `component` relevance; never a guess.
4. **Pure and dependency-light:** no I/O; workspace dependencies limited to `model`
   ([overview.md](../../../docs/architecture/overview.md#crate-dependencies)).
5. **Property tests:** ordering is total, antisymmetric, and transitive; equivalent spellings compare
   equal where the spec says so.
6. **Test vectors from real data:** advisory ID, ecosystem, version, expected affected or not, taken
   from published advisories, and the ecosystem's own conformance examples where their license
   allows. Record the source of every vector; never invent vectors.
7. **Mutation testing** pays off here: run cargo-mutants on the module you changed and review
   surviving mutants ([testing.md](../../../docs/engineering/testing.md#mutation-check)).
8. **Benchmark** the evaluation hot path
   ([performance-sensitive-change](../performance-sensitive-change/SKILL.md)).

## Verify

In `dev console`, `:upload` a real lock file of that ecosystem and `:ingest` a real advisory event
affecting one of its versions: the affected version matches at `exact-version`, a fixed one
does not.

## Done when

- [ ] Implementation follows and cites the official spec; unknown inputs are explicit.
- [ ] Property tests and sourced real-world vectors pass; surviving mutants reviewed.
- [ ] End-to-end match verified locally.
