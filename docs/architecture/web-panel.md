# Web panel and HTTP surfaces

Chat is the primary interface, but rules, routing, and linked endpoints are easier to edit in a
browser. secure-notify therefore includes one optional HTTP server that can expose independent
surfaces: the web panel, the canonical feed for federation, inbound webhooks for platforms that
need them, a health endpoint, and a metrics endpoint. All of them are disabled by default, so a default instance needs no inbound
exposure.

## Requirements

> Status: `Decided`

- The web panel is optional and disabled by default.
- Login uses a one-time link sent to the subscriber through a linked chat; there are no passwords.

## HTTP server

> Status: `Proposed`

| Surface | Path | Default | Purpose |
|---|---|---|---|
| Web panel | `/` | off | Subscriber settings and instance administration. |
| Canonical feed | `/feed/v1/...` | off | Federation (see [feed-protocol.md](feed-protocol.md)). |
| Platform webhooks | `/hooks/<platform>` | off | Inbound events for platforms that can only receive through webhooks, such as WhatsApp (see [platforms.md](platforms.md)); each request is verified with the platform's signature scheme. |
| Health | `/healthz`, `/readyz` | off | Liveness and readiness for orchestrators. |
| Metrics | `/metrics` | off | Prometheus text format; bearer token or local-only access. |

- The server binds to `127.0.0.1` by default. Exposing it publicly is done through a reverse proxy
  that terminates TLS; the server itself does not manage certificates.
- `http.public_url` MUST be configured for login links and feed URLs to be generated.

## Authentication

> Status: `Proposed`

1. The subscriber sends `/panel` from a linked interactive endpoint. For teams, only managers can
   request a link.
2. The bot replies with a single-use login link that expires after 10 minutes.
3. Opening it creates a session stored in SQLite: cookie `HttpOnly`, `Secure`, `SameSite=Strict`;
   idle timeout 24 hours, absolute lifetime 7 days. Sessions can be revoked from the panel and with
   `/panel logout`.

Admins see administration pages in the same panel; their role is checked on every request.

## Security

> Status: `Proposed`

- CSRF tokens on every state-changing form; strict Content Security Policy (`default-src 'self'`);
  no third-party scripts, fonts, or analytics.
- All output is escaped by the template engine; source content is treated as untrusted (see
  [security.md](security.md)).
- Login and feed endpoints are rate-limited.

## Pages

> Status: `Proposed`

| Area | Pages |
|---|---|
| Subscriber | Rules editor, delivery modes and routing, linked endpoints, watches, inventory (view and remove entries), watched identities, mutes, story history and search, language and timezone. |
| Admin | Sources (health, enable/disable, intervals), plugins (state, capability approvals), content updates, subscribers and invites, registration settings, federation tokens. |

## Implementation

> Status: `Proposed`

- Server-rendered HTML from templates compiled into the binary; works without JavaScript, with
  optional small progressive enhancements. No JavaScript build toolchain.
- All text goes through the i18n layer (see [delivery.md](delivery.md)).
- The footer shows the `/about` information (see [../product/licensing.md](../product/licensing.md)).
