---
name: author-skill
description: Add or change an agent skill in secure-notify's .agents/skills/ folder and keep the skills index in sync. Keep skills procedural, short, agent-agnostic, and linked to the docs that hold the rules. Use when a recurring procedure has no skill, a skill is wrong or outdated, or a doc section a skill links to has changed.
---

# Author a skill

**Use when:** creating, splitting, or editing a skill, or editing the [skills index](../README.md).
**Not for:** changing a rule; that is [change-docs](../change-docs/SKILL.md), and skills follow.

## Read first

- [workflow.md: Agent skills](../../../docs/engineering/workflow.md#agent-skills)
- [AGENTS.md](../../../AGENTS.md): ground rules 2 (agnostic) and 7 (skills hold procedures)

## Procedure

1. **Check the index.** If an existing skill covers the moment, extend it. Create a new skill only
   for a distinct phase or area, and split one that serves two unrelated moments or grows past about
   120 lines.
2. **Name it:** kebab-case, unique, describing the phase (`verify-locally`) or area
   (`platform-plugin`). Folder `.agents/skills/<name>/`, file `SKILL.md`.
3. **Front matter:**

   ```yaml
   ---
   name: <name>                 # equal to the folder name
   description: <what it does and when to load it, in one or two sentences>
   ---
   ```

   The description decides when the skill is loaded: name the concrete triggers (tasks, paths).
4. **Body sections**, in this order: title; **Use when** / **Not for** (point to the right skill
   for neighboring tasks); **Read first** (deep links to doc sections, never whole folders);
   **Procedure** (numbered, imperative steps); **Verify** or hints, when specific; **Done when**
   (checklist).
5. **Link, do not restate.** When a step applies a rule, name it briefly and link to its doc
   section. If the procedure needs a rule that does not exist, add it to `docs/` first.
6. **Stay agnostic:** no tool- or vendor-specific instructions, people, accounts, or local paths.
   Shell commands use portable syntax.
7. **Update the index:** add or edit the row in the right table, with "Load when" and the paths it
   covers; update the examples if routing changed.
8. **Check links** from the new skill (relative paths start with `../../../docs/` from a
   `SKILL.md`) and to it (index, other skills, docs).

## Done when

- [ ] Front matter `name` equals the folder; the description names its triggers.
- [ ] The skill contains procedures only, links to its rules, and stays under about 120 lines.
- [ ] The index lists it in the right table, and every link resolves.
