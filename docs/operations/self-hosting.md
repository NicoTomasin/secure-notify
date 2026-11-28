# Self-hosting

secure-notify is distributed as open-source software that anyone can run as a private instance.
Running it MUST be simple: one binary or one container, one database file, a few secrets, and no
inbound network exposure by default.

## Requirements

> Status: `Decided`

- The project is open source.
- Anyone can run their own private instance.
- Setup and configuration MUST be simple.

## Deployment shape

> Status: `Proposed`

- A single static binary, or a single container image with one data volume. A `docker-compose.yml`
  example has one service and one volume.
- Persistence in a single SQLite file inside the data directory. No external database, cache, or
  message broker.
- Outbound connections only by default: every platform plugin connects outbound (see
  [../architecture/platforms.md](../architecture/platforms.md)). The optional HTTP server is the
  only inbound surface and is disabled by default (see
  [../architecture/web-panel.md](../architecture/web-panel.md)).
- Supported targets are listed in [../engineering/releases.md](../engineering/releases.md);
  resource expectations in [../architecture/performance.md](../architecture/performance.md).

## Configuration

> Status: `Proposed`

Three layers with distinct responsibilities:

| Layer | Who | Contents |
|---|---|---|
| Environment variables (or secret files) | operator | Secrets and runtime: platform tokens (`TELEGRAM_BOT_TOKEN`, `DISCORD_BOT_TOKEN`, `SMTP_URL`, ...), `SECRET_KEY` (encryption key for watched identities), provider API keys (e.g. `NVD_API_KEY`), `DATA_DIR`, `LOG_LEVEL`, `CONFIG_FILE`, `SOURCE_URL`. |
| Instance config file (`secure-notify.toml`, optional) | operator | Enabled platforms, enabled/disabled sources and interval overrides, external plugins, plugin resource limits, content registry settings, registration settings, grace window, similarity thresholds, HTTP server surfaces and `public_url`, federation feeds, retention. Sensible defaults when absent. |
| Bot commands and web panel | each subscriber | Watches, inventory, identities, rules, delivery, endpoints, timezone, language. |

Environment variables read by the binary (every one is listed in `.env.example`; empty values count
as unset):

| Variable | Default | Meaning |
|---|---|---|
| `DATA_DIR` | `data` | Directory for the database and its backups. |
| `LOG_LEVEL` | `info` | `error`, `warn`, `info`, `debug`, or `trace`. |
| `CONFIG_FILE` | `secure-notify.toml` in `DATA_DIR`, when present | Instance config file; when set, the file MUST exist. |
| `SOURCE_URL` | the canonical repository URL | Source link shown by `/about`. |
| `TELEGRAM_BOT_TOKEN` | none | Telegram bot token (secret; `TELEGRAM_BOT_TOKEN_FILE` reads it from a file). |

- At least one platform MUST be configured.
- `SOURCE_URL` defaults to the canonical repository URL and MUST point to the operator's source if
  they run a modified version (see [../product/licensing.md](../product/licensing.md)).
- Settings for content updates are in [../architecture/content-registry.md](../architecture/content-registry.md);
  registration settings in [../architecture/subscribers.md](../architecture/subscribers.md).
- Sources and plugins that require a missing secret are disabled and listed in `/status`.

## First run

> Status: `Proposed`

1. Create a bot on at least one platform (e.g. Telegram's BotFather) and obtain its token.
2. Start the binary or container with the token set.
3. Read the one-time pairing code from the logs and send `/start <code>` to the bot. That person
   becomes the admin (see [../architecture/subscribers.md](../architecture/subscribers.md)).
4. Invite other people or teams with `/invite`, or enable open registration.
5. The first synchronization of each source runs as a backfill and does not notify (see
   [../architecture/deduplication.md](../architecture/deduplication.md)).

## Backups and upgrades

> Status: `Proposed`

- The data directory (`DATA_DIR`) holds the database file `secure-notify.db`, the `backups/`
  directory, and `tmp/` for downloads in progress (deleted when closed).
- `secure-notify backup <path>` writes a consistent online backup of the database. Copying the file
  while the instance is stopped is also valid.
- Upgrading means replacing the binary or image; migrations run automatically after an automatic
  backup (see [../engineering/migrations.md](../engineering/migrations.md)). Automatic backups
  are written to `backups/` in the data directory, and the three most recent are kept.

## Operations

> Status: `Proposed`

- `/status` (admin) and the admin pages of the web panel show each source's and plugin's state
  (disabled, first synchronization pending, healthy, or failing), last success, last error,
  consecutive failures, and items ingested; the binary and content bundle versions; pending
  capability approvals; and instance counters (active subscribers, stories, events, queued
  notifications, pending initial scans).
- Sources and plugins failing repeatedly are backed off and reported to admins, at most once per
  failure streak: every active admin is told when the consecutive failed runs reach three, and the
  streak ends with the next successful run.
- Optional `/healthz`, `/readyz`, and `/metrics` endpoints (see
  [../architecture/web-panel.md](../architecture/web-panel.md)).
- Retention defaults to one week and is configurable; the rules, including what is kept for
  deduplication, are in [../architecture/deduplication.md](../architecture/deduplication.md).
- Logs are structured (JSON) and follow the logging rules in
  [../architecture/security.md](../architecture/security.md).
