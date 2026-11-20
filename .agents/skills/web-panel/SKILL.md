---
name: web-panel
description: Change secure-notify's optional HTTP server and web panel. Covers axum routes, server-rendered templates without a JavaScript toolchain, one-time login links and SQLite sessions, CSRF and CSP, rate limits, admin pages, health and metrics endpoints, and signed platform webhooks, all disabled by default and bound to localhost. Use for anything under crates/web/.
---

# Web panel and HTTP surfaces

**Use when:** changing any HTTP surface: panel pages, login and sessions, health, metrics, platform
webhooks.
**Not for:** the canonical feed protocol semantics ([federation](../federation/SKILL.md)).

## Read first

- [web-panel.md](../../../docs/architecture/web-panel.md) (all sections)
- [security.md: Threats and mitigations](../../../docs/architecture/security.md#threats-and-mitigations)
- [delivery.md: Localization (i18n)](../../../docs/architecture/delivery.md#localization-i18n)
- [licensing.md: Source offer and `/about`](../../../docs/product/licensing.md#source-offer-and-about)
  (footer)

## Procedure

1. **Off by default,** each surface separately; the server binds to `127.0.0.1`; TLS is the reverse
   proxy's job.
2. **Server-rendered templates** compiled into the binary; every page works without JavaScript; no
   JavaScript build toolchain and no third-party scripts, fonts, or analytics.
3. **Every state-changing request** carries a CSRF token; the subscriber's identity and, for admin
   pages, the admin role are checked on every request.
4. **Escaping** is done by the template engine; never mark source-provided content as safe.
5. **Sessions:** created only from a single-use login link that expires after the documented time;
   cookie flags `HttpOnly`, `Secure`, `SameSite=Strict`; idle and absolute timeouts; revocation.
6. **Headers:** strict CSP (`default-src 'self'`); login and feed endpoints rate-limited.
7. **Platform webhooks** verify the platform's signature before parsing the body.
8. **Text** through i18n; the footer shows the `/about` information.
9. **Tests:** handler tests for authentication required, admin-only pages, CSRF rejection, expired
   and reused login links, escaping of hostile story titles, and webhook signature failures. Load
   [security-sensitive-change](../security-sensitive-change/SKILL.md).

## Verify

Run locally with the HTTP server enabled; request `/panel` on the console platform and open the
link; use the changed pages with JavaScript enabled and disabled; check response headers (CSP,
cookie flags) in the browser's developer tools or with an HTTP client; try a form without its CSRF
token; `:advance` past the login link's expiry and check the link is refused.

## Done when

- [ ] Defaults stay off and local; CSRF, CSP, escaping, and session rules hold and are tested.
- [ ] Pages work without JavaScript and were exercised in a browser.
