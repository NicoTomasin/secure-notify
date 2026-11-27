# Subscribers and access

An instance serves multiple people and teams. Each **subscriber** (a person or a team) owns one
configuration and can link **endpoints** on several platforms: a person can use Telegram and
email, a team can use a Telegram group and a Slack channel. Commands from any linked interactive
endpoint edit the same configuration, and deduplication is per subscriber, so a story is delivered
once per subscriber no matter how many endpoints it has.

## Multi-subscriber instance

> Status: `Decided`

One instance MUST support multiple subscribers, each with an independent configuration.

## Subscribers and endpoints

> Status: `Decided` (one subscriber, many linked endpoints; persons and teams) / `Proposed` (details)

- A subscriber is of kind `person` or `team`. It owns watches, inventory, watched identities,
  rules, mutes, delivery preferences, timezone, and language.
- An endpoint is one destination on one platform, linked to exactly one subscriber. Person
  subscribers link private endpoints (private chats, direct messages, email addresses, webhooks);
  team subscribers link group endpoints (group chats, channels) and MAY also link email addresses
  and webhooks.
- Every subscriber has one **primary endpoint**, which receives notifications unless routing says
  otherwise (see [delivery.md](delivery.md)).
- A subscriber MUST have at least one active interactive endpoint, or the web panel enabled, to be
  configurable. Delivery-only endpoints (email, webhook) cannot run commands.
- When the bot loses access to an endpoint (removed from a group, user blocked the bot), that
  endpoint is deactivated. A subscriber with no active endpoints is deactivated and its queue is
  dropped; its configuration is kept until an admin removes it or it is reactivated.

## Linking endpoints

> Status: `Proposed`

1. From an interactive endpoint already linked to the subscriber, run `/link`. The bot replies with
   a single-use link code valid for 10 minutes.
2. On the new endpoint, send `/link <code>` (interactive platforms) or add the endpoint from the
   original chat (`/link email <address>`, `/link webhook <url>`), which sends a verification code
   or test request to the new endpoint.
3. Once verified, the endpoint is linked. `/link` without arguments also lists linked endpoints;
   `/unlink <endpoint>` removes one (never the last interactive endpoint).

For team subscribers, only a manager (see "Permissions in groups") can link or unlink endpoints.

## Joining an instance

> Status: `Decided` (invite by default, optional open registration) / `Proposed` (details)

- **Pairing.** On first boot the instance prints a one-time pairing code to its logs. Until an
  admin exists, every start prints the same code again. The first
  person who sends `/start <code>` from any interactive platform becomes the **admin**.
- **Invites.** Admins create single-use, expiring invite codes with `/invite` (for a person or for
  a team). An invite is valid for seven days. A person redeems one in a private chat with `/start <code>`; a team redeems one by
  adding the bot to a group and sending `/start <code>` there (by a group administrator).
- **Open registration.** Disabled by default. When an admin enables it, people can join with
  `/start` alone, within limits set in the instance config:

  | Setting | Meaning |
  |---|---|
  | `registration.mode` | `invite` (default) or `open` |
  | `registration.max_subscribers` | Cap on active subscribers |
  | `registration.platforms` | Platforms on which open registration is accepted |
  | `registration.teams` | Whether teams can join without an invite |
  | `registration.approval` | `none` or `manual` (admins approve each request) |
  | `registration.rate_limit` | Maximum new registrations per hour |

- Pairing and invite codes are ten characters from the Crockford base32 alphabet, generated with a
  cryptographically secure random source, and accepted in any letter case.
- Messages from chats that are not registered endpoints MUST be ignored, except `/start` with a
  valid code (or `/start` alone in open mode). No data leakage, no command execution.

## Roles and permissions

> Status: `Proposed`

- Instance roles belong to persons only: `admin` or `member`. Admins can promote other persons to
  admin. Teams never hold instance-level permissions.
- Admin commands: `/invite`, `/subscribers`, `/remove`, `/status`, and registration management
  (see [platforms.md](platforms.md)).
- The operator has access to the database. The onboarding message MUST state what is stored (see
  [inventory.md](inventory.md) and [exposure-monitoring.md](exposure-monitoring.md)).

### Permissions in groups

> Status: `Proposed`

- A team's **managers** are the platform-level administrators of any of its linked group
  endpoints, checked through the platform's API at command time.
- Only managers may change team settings, link endpoints, or upload files. Any member of a linked
  group may run read-only commands.
- Platforms that only deliver commands and replies to bots in groups (e.g. Telegram's privacy
  mode) require uploads as a reply to a bot prompt (`/upload`). The exact flow per platform is
  described in [platforms.md](platforms.md).
