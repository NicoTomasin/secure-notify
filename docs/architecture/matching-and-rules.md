# Matching, severity, and rules

For every story that is new or materially changed, the matcher decides, per subscriber, whether it
is relevant and whether it passes the subscriber's mutes and rules. Relevance (how directly it
affects the subscriber) and severity (how serious it is) are independent axes, and rules combine
both with the story's kind, entities, components, source, and exploitation signals.

## Requirements

> Status: `Decided`

- Subscribers MUST be able to configure the minimum severity they want to be notified about.
- Subscribers MUST be able to exclude specific sources; this is done with rules that match on
  `source`. Enabling and disabling sources for the whole instance remains an admin setting.

## Relevance

> Status: `Proposed`

Computed per subscriber; the highest level wins.

| Level | Condition |
|---|---|
| `identity` | One of the subscriber's verified watched identities appears in the story's breach (see [exposure-monitoring.md](exposure-monitoring.md)). |
| `exact-version` | A story's `affected` entry matches a component in the subscriber's inventory **and** that component's exact version is inside the affected range. |
| `component` | A story's `affected` component is in the inventory but its version is unknown (manifest with ranges only, or product added without version). |
| `product` | The story is tagged with a product or vendor the subscriber watches (watching a vendor includes its products). |
| `organization` | The story is tagged with an organization the subscriber watches (breach, attack, or outage at a service they use). |
| `technology` | The story is tagged with a technology the subscriber watches. |
| none | Not relevant; nothing is queued. |

If the inventory has the component at an exact version that is **not** affected, the story is not
relevant at the `exact-version` or `component` level for that component.

## Matching at scale

> Status: `Proposed`

The matcher MUST NOT iterate over all subscribers for each story. It uses inverted indexes kept in
sync with subscriber data: component → subscribers, entity → subscribers, identity exposure →
subscriber. Budgets are in [performance.md](performance.md).

## Version evaluation

> Status: `Proposed`

- Exact-version checks are evaluated locally, never through third-party APIs (see
  [inventory.md](inventory.md)).
- Range semantics are implemented per ecosystem in `crates/versions`: semver (npm, Cargo, and
  others), PEP 440 (Python), Maven, Go modules, RubyGems, NuGet, Debian, RPM, Alpine, and a generic
  ordering for CPE products.
- Ecosystems without a reliable implementation yield `component` relevance instead of guessing.
- Every implementation is covered by property tests and by test vectors taken from real advisories.

## Severity normalization

> Status: `Decided` (`malicious-package` default) / `Proposed` (rest)

Vulnerability family:

| Input | Level |
|---|---|
| Explicit level from source (e.g. GHSA severity) | Used as-is (mapped to the five levels). |
| `malicious-package` event without an explicit level | `critical`: a malicious package in an inventory is always critical. |
| CVSS 9.0–10.0 | `critical` |
| CVSS 7.0–8.9 | `high` |
| CVSS 4.0–6.9 | `medium` |
| CVSS 0.1–3.9 | `low` |
| Neither, declarative source with `defaults.severity` | That default. |
| Neither | `unknown`, optionally raised by deterministic keyword rules from content (e.g. "remote code execution", "actively exploited"). |

When several CVSS versions exist, the stored score prefers v4.0, then v3.1, v3.0, v2.0. EPSS does
not change the level; rules can use it.

Breaches (base level from the most sensitive data class, raised one level when `records` is at
least 10,000,000, up to `critical`):

| Data classes | Level |
|---|---|
| `password`, `auth-token`, `payment-card`, `financial`, `government-id`, `health`, `biometric` | `critical` |
| `password-hash`, `security-qa`, `private-messages`, `location`, `date-of-birth` | `high` |
| `name`, `phone`, `address`, `ip-address` | `medium` |
| `email`, `username`, `other` only | `low` |
| None known | `unknown` |

Attacks and outages: incident `impact` `critical` → `critical`, `major` → `high`, `minor` →
`medium`, `none` → `low`; missing impact → `unknown`, optionally raised by keyword rules.
Scheduled maintenance (`status = scheduled`) is `low`.

A story's severity is the highest level among its events (see [data-model.md](data-model.md)).

## Severity thresholds

> Status: `Decided`

- Levels are ordered `critical` > `high` > `medium` > `low`.
- In every threshold comparison (a rule's `min_severity`, the [minimum severity](#minimum-severity)),
  `unknown` counts as `medium`: a story of unknown severity passes a `medium` threshold and does not
  pass a `high` one.
- Thresholds do not change the stored level: a story of unknown severity is still shown and
  delivered as `unknown` (see [delivery.md](delivery.md)).

## Rules

> Status: `Proposed`

Rules are evaluated in order; the first matching rule decides. Subscribers edit rules through the
bot or the web panel; TOML is shown for clarity:

```toml
[[rule]]
match = { exploited = true }
action = "notify"                 # bypasses severity thresholds

[[rule]]
match = { source = "some-noisy-blog" }
action = "ignore"                 # excluding a source

[[rule]]
match = { entity = "tech:php" }
action = "notify"
min_severity = "critical"         # per-entity override

[[rule]]
match = { relevance = "exact-version" }
action = "notify"
min_severity = "medium"
```

Match fields (all optional, combined with AND):

| Field | Matches |
|---|---|
| `relevance` | A relevance level, or a list of them. |
| `kind` | An event kind or list (see [../product/scope.md](../product/scope.md)). |
| `entity` | A taxonomy ID; a trailing `*` matches a prefix (e.g. `product:fortinet/*`). |
| `component` | A purl or CPE prefix. |
| `source` | A source ID or list. |
| `exploited`, `exploit_available` | Boolean flags. |
| `min_epss` | Minimum EPSS score (0–1). |

Actions: `notify` (optionally with `min_severity`) or `ignore`. Mutes (story, entity, component,
or component version; optionally time-limited) are applied before rules, and the
[minimum severity](#minimum-severity) after them. A story that matches no rule is not notified.

## Minimum severity

> Status: `Decided`

- Each subscriber MAY set a minimum severity. It is unset by default.
- It applies after the rules: a story that a rule notifies is still not notified when its severity
  is below the minimum (compared as in [Severity thresholds](#severity-thresholds)), unless the
  story is `exploited`.
- Subscribers set it with `/rules severity <level>` and remove it with `/rules severity off` (see
  [platforms.md](platforms.md#command-set)).

## Default rules for new subscribers

> Status: `Proposed`

New subscribers start with this rule set, so the bot is useful without configuration. Subscribers
can reset to defaults at any time.

| Order | Match | Action |
|---|---|---|
| 1 | `exploited = true` | notify |
| 2 | `relevance = identity` | notify |
| 3 | `relevance = exact-version` | notify, `min_severity = medium` |
| 4 | `relevance = component` | notify, `min_severity = high` |
| 5 | `relevance = product` | notify, `min_severity = high` |
| 6 | `relevance = organization`, `kind = [breach, attack]` | notify, `min_severity = medium` |
| 7 | `relevance = organization`, `kind = outage` | notify, `min_severity = high` |
| 8 | `relevance = technology` | notify, `min_severity = high` |

Relevance is always required: rules filter relevant stories, they never make an irrelevant story
relevant.
