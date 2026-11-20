---
name: i18n
description: Add or change user-facing text in secure-notify through the Fluent i18n layer, or contribute a translation. Covers English reference keys, variables and plurals, untranslated source content, the never-translated attribution line, and per-locale rendering snapshots. Use when touching bot replies, notifications, digests, buttons, web panel text, or files under locales/.
---

# User-facing text and translations

**Use when:** adding or changing any text a subscriber or admin sees, or editing `locales/`.
**Not for:** log messages and code identifiers, which stay in English and outside the i18n layer.

## Read first

- [delivery.md: Localization (i18n)](../../../docs/architecture/delivery.md#localization-i18n)
- [licensing.md: Source offer and `/about`](../../../docs/product/licensing.md#source-offer-and-about)

## Procedure

1. **No hard-coded user-facing strings.** Add a message to the English reference file in
   `locales/en/`, following the naming pattern already used in that file.
2. **Use Fluent features** instead of string assembly: variables for values, selectors for plurals
   and variants. Never concatenate translated fragments; word order differs between languages.
3. **Source content** (titles, summaries, URLs) is passed through as a variable, untranslated, and
   escaped by the platform renderer.
4. **Attribution:** the attribution line and the license name in `/about` are never translated or
   removed.
5. **Translations:** copy the keys from `locales/en/`, keep every variable, translate only the text.
   Missing keys fall back to English; a partial locale is acceptable.
6. **Platform limits:** check that the longest translation still fits the message limits, or that
   splitting handles it ([platforms.md: Capabilities](../../../docs/architecture/platforms.md#capabilities)).
7. **Tests:** rendering snapshots per platform × locale for changed messages
   ([testing.md: Snapshots](../../../docs/engineering/testing.md#snapshots)); review every changed
   line.

## Verify

On the console platform, switch `/language` and trigger the changed messages; check variables,
plurals, and fallback for a key missing in the target locale.

## Done when

- [ ] Every new user-facing string is a Fluent message in `locales/en/`.
- [ ] No concatenated translations; attribution untouched.
- [ ] Snapshots reviewed for the affected platforms and locales.
