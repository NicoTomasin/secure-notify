---
name: change-docs
description: Change secure-notify's docs or rules correctly. Find the single home of a rule, apply the contradiction protocol, set status labels, add docs and glossary terms, resolve open questions, and keep links and dependent skills in sync. Use for any edit under docs/, AGENTS.md, or root community files, and before implementing a behavior change.
---

# Change docs and rules

**Use when:** a change alters behavior, architecture, or a rule; you edit `docs/`, `AGENTS.md`,
`README.md`, `CONTRIBUTING.md`, `SECURITY.md`; or you resolve an open question.
**Not for:** editing skills only ([author-skill](../author-skill/SKILL.md)).

## Read first

- [AGENTS.md](../../../AGENTS.md): ground rules, contradiction protocol, status labels.
- [docs/README.md: Writing conventions](../../../docs/README.md#writing-conventions)

## Procedure

1. **Find the rule's single home.** Search for the term (`grep -rn "<term>" docs AGENTS.md`) and
   check the [glossary](../../../docs/product/glossary.md). If the rule already lives somewhere, edit
   it there and link to it from elsewhere; never restate it.
2. **Classify the change:**
   - Clarification with no behavior change: edit directly.
   - Change to a `Decided` rule: run the contradiction protocol. Ask the maintainer; if the new rule
     wins, delete the old text entirely, write the new rule, update every doc that links to it, and
     mark it `Decided`.
   - Change to a `Proposed` rule: allowed; list it in the pull request.
   - New rule written by you: label it `Proposed` and list it in the pull request. Only a
     maintainer's explicit confirmation makes it `Decided`.
   - Affects an `Open` question: ask. Once answered, write the answer into the doc and delete the
     row from [open-questions.md](../../../docs/open-questions.md).
3. **Write** following the conventions: summary paragraph first in new docs, one status label per
   rule section, RFC 2119 keywords, current state only (no history, no "previously"), English, no
   names, accounts, local paths, or vendors. The repository URL appears only in
   [licensing.md](../../../docs/product/licensing.md) and `NOTICE`.
4. **New doc:** add it to the layout in [docs/README.md](../../../docs/README.md) and to the table
   in [AGENTS.md](../../../AGENTS.md). **New term:** add it to the glossary first.
5. **Renamed or removed heading:** find every link to its anchor, including from `.agents/skills/`
   (`grep -rn "<file>.md#<anchor>" docs .agents AGENTS.md`), and update them.
6. **Dependent skills:** if a procedure changes because of the rule, update the skills that link to
   the section in the same change ([author-skill](../author-skill/SKILL.md)).

## Verify

- Every link you added or touched resolves, including anchors (the docs link check in CI; locally,
  `cargo xtask ci` once it exists, or any Markdown link checker).
- Read the rendered Markdown of every changed file.

## Done when

- [ ] Each rule lives in exactly one place; other places link to it.
- [ ] Every rule section carries the right label; new rules are `Proposed` and listed for the PR.
- [ ] Indexes, glossary, links, and dependent skills are updated.
