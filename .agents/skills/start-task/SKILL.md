---
name: start-task
description: First step of every task in secure-notify. Understand the request, find the governing docs and their status labels, detect contradictions and ethical or scope conflicts, choose the skills to load, plan the verification, and create the branch. Use at the start of any change.
---

# Start a task

**Use when:** you are about to begin any change: docs, content, plugin, core, or infrastructure.
**Not for:** continuing a task whose docs and branch you already have.

## Read first

- [AGENTS.md](../../../AGENTS.md): already loaded. Ground rules, contradiction protocol, labels.
- [workflow.md: Lifecycle of a change](../../../docs/engineering/workflow.md#lifecycle-of-a-change)
- [Skills index](../README.md)

## Procedure

1. **Restate the task** in one sentence and classify it: docs, content (feeds, taxonomy), plugin,
   core, or infrastructure. If an ambiguity would change what you build, ask before going further.
2. **Pick the skills.** From the skills index, choose the area skill(s) that match the paths you
   will touch. Load only those. Follow their "Read first" links; read sections, not whole docs.
3. **Check the status label** of every doc section you will rely on:
   - `Decided`: binding.
   - `Proposed`: build on it; if it materially shapes your implementation, note it for the pull
     request.
   - `Open` (listed in [open-questions.md](../../../docs/open-questions.md)): stop and ask.
4. **Look for contradictions.** If the request conflicts with a documented rule, run the
   contradiction protocol in [AGENTS.md](../../../AGENTS.md) before writing anything.
5. **Check scope and ethics** against [vision.md: Non-goals](../../../docs/product/vision.md#non-goals)
   and [Ethical boundaries](../../../docs/product/vision.md#ethical-boundaries): no LLM in the core,
   no leaked data, no offensive capability, no telemetry. A request that crosses them is raised with
   the human, not implemented.
6. **Plan.** List the files you expect to touch and the docs that must change with them (ground
   rule 6). Keep the change to one logical unit; split larger work into several pull requests. If
   the plan spans areas, sequence them and load each area skill only when you reach it.
7. **Plan the verification now.** Look up your change type in
   [workflow.md: Local verification](../../../docs/engineering/workflow.md#local-verification). If
   you will not be able to perform it (missing token, hardware, account), tell the human at the
   start, not at the end.
8. **Branch.** Load [git-workflow](../git-workflow/SKILL.md) and create the branch from the right
   base.

## Done when

- [ ] You can name the doc sections that govern the change and their labels.
- [ ] No unresolved contradiction or `Open` question blocks the work.
- [ ] You know which skills to load next, and which not to.
- [ ] The verification plan is known and any blocker was communicated.
- [ ] You are on a correctly named branch from the right base.
