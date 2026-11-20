---
name: matching
description: Change how secure-notify decides, per subscriber, whether a story is relevant and passes their filters. Covers relevance levels, inverted indexes, mutes, first-match rule evaluation, default rules, and severity normalization for vulnerabilities, breaches, attacks, and outages. Use for matcher and rules code in crates/core.
---

# Matching, rules, and severity

**Use when:** changing relevance computation, the inverted indexes, mutes, rule evaluation, default
rules, or severity mapping.
**Not for:** version range semantics ([version-ranges](../version-ranges/SKILL.md)) or when
notifications are sent ([delivery](../delivery/SKILL.md)).

## Read first

- [matching-and-rules.md](../../../docs/architecture/matching-and-rules.md) (all sections)
- [scope.md: Watch targets](../../../docs/product/scope.md#watch-targets)
- [performance.md: Budgets](../../../docs/architecture/performance.md#budgets) (matching latency)

## Procedure

1. **Docs first** for any change in rule semantics, relevance levels, default rules, or severity
   tables ([change-docs](../change-docs/SKILL.md)).
2. **Relevance before rules:** rules only filter relevant stories; they never make an irrelevant
   story relevant.
3. **Indexes, never scans:** component → subscribers, entity → subscribers, identity exposure →
   subscriber, kept in sync when inventory, watches, or identities change.
4. **Evaluation order:** mutes first, then rules in order, first match decides; a story that matches
   no rule is not notified.
5. **Severity:** apply the tables exactly, including the CVSS version preference and the breach
   record-count rule; EPSS never changes the level.
6. **Versions:** exact-version checks go through `crates/versions`; an ecosystem without reliable
   semantics yields `component` relevance.
7. **Tests:** table-driven tests with one case per row of the doc tables (severity mapping,
   relevance levels, default rules); a version present but not affected yields no match; mutes beat
   rules; benchmark for the matching budget
   ([performance-sensitive-change](../performance-sensitive-change/SKILL.md)).

## Verify

In `dev console`, as two users: `:upload` a lock file containing a version affected by a real
advisory for one, and a fixed version for the other; watch a technology; `:ingest` the advisory's
event and `:advance` past the grace window. Check that the affected subscriber gets a notification
with the right relevance reason, the fixed one gets nothing, and a mute suppresses it.

## Done when

- [ ] Doc tables and code agree; each table row has a test case.
- [ ] Matching uses indexes and stays within budget.
- [ ] The end-to-end check on the console platform is recorded.
