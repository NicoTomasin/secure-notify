# Governance

How decisions are made in secure-notify, who makes them, how people gain responsibility, and how
the project is funded. secure-notify is a non-profit community project: it starts led by its
founding maintainer and moves to a maintainer team as sustained contributors join.

## Decision model

> Status: `Decided`

- The project is maintainer-led. While there is a single maintainer, that maintainer decides.
- When there are sustained contributors, new maintainers are added and decisions move to the
  maintainer team.

## Roles

> Status: `Proposed`

| Role | Can | Becomes one by |
|---|---|---|
| Contributor | Open issues and pull requests, review, discuss. | Contributing anything. |
| Reviewer | Approve pull requests in an area (e.g. content, translations, a platform). | Sustained, high-quality contributions in that area; nominated by a maintainer. |
| Maintainer | Merge, release, change `Decided` rules, manage the repository and the content signing environment. | Sustained contributions over at least three months and nomination by a maintainer, approved by the existing maintainers. |

Maintainers and reviewers who are inactive for twelve months become emeritus and can return on
request.

## How decisions are made

> Status: `Proposed`

- Docs are the source of truth. Changes to rules follow the contradiction protocol in
  [`../../AGENTS.md`](../../AGENTS.md); a change to a `Decided` rule MUST be approved by a
  maintainer in an issue or pull request before it is merged.
- With a maintainer team, decisions are made by lazy consensus: a proposal passes if no maintainer
  objects within seven days. Objections are discussed; if consensus fails, maintainers vote and a
  simple majority decides. The founding maintainer breaks ties until the team adopts a different
  tie-break rule in this doc.
- Decisions and their rationale are recorded in the issue or pull request that made them; the docs
  record only the result.

## Funding

> Status: `Decided`

- The project accepts donations through GitHub Sponsors. The maintainers publish how the funds are
  spent.
- Funds are used only for shared infrastructure: content registry hosting, CI, and API keys needed
  for testing sources.
- The project never sells paid features, priority support, or a "pro" edition.

## Code of conduct

> Status: `Proposed`

The project adopts the Contributor Covenant, version 2.1 (see
[`../../CODE_OF_CONDUCT.md`](../../CODE_OF_CONDUCT.md)). Maintainers enforce it.

## Communication

> Status: `Proposed`

- Issues for bugs, source requests, and proposals; pull requests for changes; repository
  discussions for questions and ideas.
- Security issues are never discussed in public before a fix (see
  [`../../SECURITY.md`](../../SECURITY.md)).
