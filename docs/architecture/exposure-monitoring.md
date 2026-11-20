# Exposure monitoring

Subscribers can ask the bot to watch their own email addresses and domains and to warn them when
one appears in a known breach. Unlike the inventory, a watched identity is personal data, so this
feature is opt-in, requires proof of ownership, is encrypted at rest, and is disabled unless the
operator configures an exposure provider.

## Rules

> Status: `Decided`

- Watching an identity is opt-in: nothing is watched unless the subscriber adds it explicitly.
- Ownership MUST be verified before an identity is watched.
- Watched identities MUST be encrypted at rest and MUST NOT appear in logs.
- The feature is disabled unless the operator enables at least one exposure provider.
- secure-notify reports that an identity appears in a breach and which kinds of data were exposed,
  never the leaked values (see the ethical boundaries in [../product/vision.md](../product/vision.md)).

## Identity kinds

> Status: `Proposed`

| Kind | Who can watch it | Verification |
|---|---|---|
| Email address | Person subscribers only | A code sent to the address, entered in the bot. Requires the email platform to be configured (see [platforms.md](platforms.md)). |
| Domain | Person and team subscribers | A DNS TXT record `secure-notify-verify=<token>` on the domain, checked by the core. |

- Verification codes are single-use and expire after 15 minutes. DNS verification tokens expire
  after 7 days if not confirmed.
- Domains are re-verified periodically (default monthly); an identity whose verification lapses
  stops being checked until it is verified again.

## Storage

> Status: `Proposed`

- `value_encrypted`: the identity encrypted with an instance key (authenticated encryption), so
  the plaintext is available only when a check runs.
- `value_hash`: a keyed hash used for uniqueness and lookups, so the plaintext is never needed for
  queries.
- The instance key comes from the operator's configuration, never from the database (see
  [../operations/self-hosting.md](../operations/self-hosting.md)). [security.md](security.md)
  describes what this protects against and what it does not.
- Removing an identity (`/identities remove`) deletes it and all its exposure records immediately.

## Checks

> Status: `Proposed`

- An identity is checked when it is verified, on a schedule (default daily, respecting provider
  rate limits), and when a breach source reports a new breach.
- Checks go through exposure provider plugins (see [plugins.md](plugins.md)), which return the
  breach identifiers an identity appears in.
- Each finding is stored in `identity_exposures` and linked to the breach story with the same
  identifier (creating a minimal story if no breach source has reported it yet).
- Only new exposures produce notifications, with relevance `identity` (see
  [matching-and-rules.md](matching-and-rules.md)). The first check after verification sends one
  summary of existing exposures instead of one notification per breach.

## Providers

> Status: `Decided` (as many free providers as possible) / `Proposed` (details)

- The project supports as many exposure providers as possible, following the free-access rule in
  [../product/sources-catalog.md](../product/sources-catalog.md): free APIs or public lookups whose
  terms allow automated use. Providers that turn out to require payment are dropped.
- Candidates are listed in the sources catalog.
- Providers receive only the identity being checked, over TLS, and only from the exposure provider
  plugin that declared that provider's host.
- When several providers are enabled, an identity is checked against all of them and findings are
  merged by breach identifier.

## Commands

> Status: `Proposed`

`/identities` lists watched identities and their verification state; `/identities add <email or
domain>` starts verification; `/identities verify <code>` completes email verification;
`/identities remove <value>` deletes an identity. The command set is in [platforms.md](platforms.md).
