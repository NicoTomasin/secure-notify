---
name: security-sensitive-change
description: Checklist for secure-notify changes that touch the attack surface or sensitive data. Covers fetched content, uploads, plugins and capabilities, the shared HTTP client and SSRF, secrets, personal data and logs, authentication, the web panel, and ethical boundaries. Use alongside the area skill whenever one of these is involved.
---

# Security-sensitive change

**Use when:** your change parses untrusted input, handles personal data or secrets, performs
network requests, enforces capabilities or permissions, or touches authentication or sessions.
**Not for:** reporting a vulnerability in released code; follow [SECURITY.md](../../../SECURITY.md)
and do not disclose it publicly.

## Read first

- [security.md](../../../docs/architecture/security.md) (all sections)
- [inventory.md: Privacy](../../../docs/architecture/inventory.md#privacy)
- [exposure-monitoring.md: Rules](../../../docs/architecture/exposure-monitoring.md#rules)
- [vision.md: Ethical boundaries](../../../docs/product/vision.md#ethical-boundaries)

## Checklist

**Untrusted input** (fetched content, uploads, feed responses, plugin output)
- [ ] Size and time limits apply before parsing; large inputs are streamed.
- [ ] Output is validated against its schema; invalid input is rejected without a panic.
- [ ] Nothing from content is executed or evaluated.
- [ ] Rendered content is escaped for the target format; only `http`/`https` links are rendered.

**Network**
- [ ] All requests go through the shared HTTP client (rate limits, timeouts, size limits, SSRF guard
  after redirects and DNS resolution).
- [ ] A plugin reaches only hosts declared in its manifest; no direct sockets.
- [ ] No subscriber or operator data is sent to third parties; inventory never leaves the instance.

**Secrets and personal data**
- [ ] Secrets come from the environment or secret files, in redacting types
  ([configuration.md](../../../docs/engineering/configuration.md#secrets-in-code)).
- [ ] Uploaded files are not stored; watched identities are encrypted, hashed, opt-in, and verified.
- [ ] Logs contain none of the forbidden data; endpoints appear only as internal IDs.

**Permissions and plugins**
- [ ] Commands check permissions at execution time; unknown chats are ignored.
- [ ] Plugin capabilities are minimal; capability expansions wait for admin approval.

**Web**
- [ ] CSRF tokens, strict CSP, template escaping, cookie flags, rate limits
  ([web-panel.md: Security](../../../docs/architecture/web-panel.md#security)).

**Ethics**
- [ ] No connection to leak sites or underground forums, no leaked values, no exploit code, no
  telemetry.

## Tests

Add adversarial cases next to the normal ones: oversized and deeply nested input, malformed
encodings, `javascript:` and private-address links, redirects to private addresses, markup
injection in titles, undeclared hosts, and assertions that forbidden data is absent from logs and
storage. Synthetic adversarial fixtures are marked as such
([testing.md: Fixtures](../../../docs/engineering/testing.md#fixtures)).

## Done when

- [ ] Every applicable checklist item holds, with tests for the adversarial cases.
- [ ] The pull request names the security-relevant aspects for the reviewer.
