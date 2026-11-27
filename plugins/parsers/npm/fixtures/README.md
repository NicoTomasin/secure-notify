# Fixtures: npm parser

Real files from public open-source repositories, retrieved on 2026-11-27. Files marked "trimmed"
keep the original structure with fewer entries (reserialized with two-space indentation); no
values were changed.

| File | Origin | Commit | Notes |
|---|---|---|---|
| `package-lock-v3.json` | `octokit/core.js`, `package-lock.json` | `6432a6fa68e89c296e927cbbb6c02f16a1040ba1` | Trimmed: root entry and four packages, one nested under `node_modules/@octokit/auth-app/`. |
| `package-lock-v1.json` | `nodejs/nodejs.org`, `package-lock.json` | `3777347dd152ff352f929b3030786f15c9c7ab44` | Trimmed: three top-level dependencies, two with nested dependencies. |
| `npm-shrinkwrap-v3.json` | `klaudworks/ralph-meets-rex`, `npm-shrinkwrap.json` | `8d95260a35b03f7f5b298efe744ab2c9f8c72390` | Complete. |
| `npm-shrinkwrap-v2.json` | `johnwalicki/node-red-contrib-twc-weather`, `npm-shrinkwrap.json` | `8463cf8b04a4e23e7f0350749bce2d8e70e43e2a` | Complete; carries both `packages` and the legacy `dependencies` map. |
| `package.json` | `Rich-Harris/estree-walker`, `package.json` | `8f7c1ebe458461ce9018d3f08464fb6fc0ac4bce` | Complete. |

Personal data was replaced with reserved names: the `author` and `repository.url` of
`package.json` and the sponsor URLs in `funding` (`https://example.com/...`).

Synthetic inputs (non-registry specifiers, malformed and deeply nested JSON) are written inline in
`tests/fixtures.rs` and marked as synthetic there.
