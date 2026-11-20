---
name: exposure-monitoring
description: Change secure-notify's exposure monitoring of watched email addresses and domains. Covers opt-in and ownership verification (email codes, DNS TXT records), authenticated encryption and keyed hashing at rest, scheduled checks, exposure provider plugins with a single declared host and no state, identity notifications, and immediate deletion. Use for anything that handles watched identities or lives under plugins/exposure/.
---

# Exposure monitoring

**Use when:** changing identity verification, storage, checks, notifications, `/identities`, or an
exposure provider plugin.
**Not for:** breach sources that report breaches in general
([declarative-source](../declarative-source/SKILL.md), [source-plugin](../source-plugin/SKILL.md)).

## Read first

- [exposure-monitoring.md](../../../docs/architecture/exposure-monitoring.md) (all sections)
- [plugins.md: Exposure providers](../../../docs/architecture/plugins.md#exposure-providers)
- [security.md: Secrets](../../../docs/architecture/security.md#secrets)
- [vision.md: Ethical boundaries](../../../docs/product/vision.md#ethical-boundaries)
- [sources-catalog.md: Free access](../../../docs/product/sources-catalog.md#free-access)

## Procedure

1. **The `Decided` rules frame everything:** opt-in, ownership verified before watching, encrypted at
   rest, never logged, feature off without a provider, never the leaked values. Load
   [security-sensitive-change](../security-sensitive-change/SKILL.md).
2. **Storage:** authenticated encryption with the instance key from configuration
   ([config-and-secrets](../config-and-secrets/SKILL.md)) plus a keyed hash for lookups. Plaintext
   exists only in memory during verification and checks, in redacting types.
3. **Verification:** single-use, expiring email codes through the email platform; DNS TXT checks
   through the core; periodic domain re-verification; a lapsed identity stops being checked.
4. **Providers:** exactly one declared host, no persistent state, free access only; they receive only
   the identity being checked and return breach identifiers.
5. **Findings** go to `identity_exposures`, linked to the breach story with the same identifier
   (creating a minimal story when needed). The first check after verification sends one summary;
   later, only new exposures notify, with relevance `identity`.
6. **Deletion:** removing an identity deletes it and its exposure records immediately.
7. **Tests:** only reserved names (`user@example.com`, `example.org`) and recorded provider responses
   with identifying values replaced by reserved ones
   ([testing.md: Fixtures](../../../docs/engineering/testing.md#fixtures)); assertions that the
   plaintext appears in no log line and no unencrypted column; code expiry and reuse.

## Verify

- With the email platform pointed at a local SMTP test server, add an identity, receive the code,
  verify, and run a check against a provider's recorded or live response.
- Check code expiry with `:advance` in `dev console`.
- Domain verification needs a domain you control; if you have none, state it as not verified.
- Inspect the database: only `value_encrypted` and `value_hash` hold the identity.

## Done when

- [ ] Every `Decided` rule holds and is tested; no plaintext in logs or storage.
- [ ] Providers declare one host and keep no state; fixtures contain no personal data.
- [ ] Verification and check flows verified locally, or explicitly declared not verified.
