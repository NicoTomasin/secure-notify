# Licensing

secure-notify is free software under the GNU Affero General Public License v3 or later
(AGPL-3.0-or-later), with one additional term: derivative works must credit the original project
by name and repository URL. Anyone may use, modify, and even sell it, but whoever distributes it
or runs a modified version as a service must publish their complete source under the same
license, and keep the attribution. The pieces that plugin authors build against (SDKs, interface
definitions, schemas) are licensed permissively, and an additional permission lets plugins that
use only the published interfaces carry any license.

## License

> Status: `Decided`

- The core, the built-in plugins, content, and translations are licensed under
  **AGPL-3.0-or-later**. The full text is in [`../../LICENSE`](../../LICENSE).
- Copyright holder line: `The secure-notify contributors`.
- The project MUST remain OSI open source. Do not add terms that restrict use or sale (e.g.
  non-commercial clauses), because they would break this.

## Repository URL

> Status: `Decided`

- The canonical repository URL is **`https://github.com/NicoTomasin/secure-notify`**.
- The repository stays under this personal namespace; moving it to an organization is
  reconsidered only if the project grows.
- This doc and [`../../NOTICE`](../../NOTICE) are the only places where the URL is written out.
  Other docs refer to "the canonical repository URL" and link here.
- Values derived from it (the `SOURCE_URL` default, the source User-Agent, the default content
  registry location) are documented where they are used.

## Attribution (additional term, AGPL section 7(b))

> Status: `Decided`

- Every copy and derivative work MUST preserve the attribution
  `Based on secure-notify (<canonical repository URL>)`, as defined in
  [`../../NOTICE`](../../NOTICE).
- The attribution names the project and its repository URL, not any individual.

## Permissively licensed parts

> Status: `Decided` (MIT OR Apache-2.0 for plugin-facing parts) / `Proposed` (exact paths)

Plugin authors MUST be able to build plugins without being bound by the AGPL. The following parts
are licensed **MIT OR Apache-2.0**:

| Path | Contents |
|---|---|
| `sdk/` | WIT packages (plugin interfaces) and per-language guest SDKs. |
| `schemas/` | JSON Schemas (normalized event, canonical feed, plugin manifest, declarative source, content bundle). |
| `crates/model/` | Canonical Rust types from which the JSON Schemas are generated. |

- The license texts are in [`../../LICENSE-MIT`](../../LICENSE-MIT) and
  [`../../LICENSE-APACHE`](../../LICENSE-APACHE).
- Implementing the canonical feed protocol (see
  [../architecture/feed-protocol.md](../architecture/feed-protocol.md)) requires no license.
- Plugins inside this repository are AGPL-3.0-or-later like the rest of the repository.

## Plugin interface exception

> Status: `Decided`

- WASM plugins run inside the AGPL-licensed process. An additional permission (AGPL section 7) in
  [`../../NOTICE`](../../NOTICE) allows conveying secure-notify combined with plugins that interact
  with it only through the published WIT interfaces, without those plugins having to be licensed
  under the AGPL.
- The legal text lives only in `NOTICE`. It was adopted without a formal legal review.

## Source offer and `/about`

> Status: `Proposed`

AGPL section 13 requires that users who interact with a modified version over a network can get
its source. For a bot:

- The bot MUST provide an `/about` command, available to every subscriber on every interactive
  platform, that shows the attribution, the license (AGPL-3.0-or-later), the running version, and
  a link to the source code. The web panel, when enabled, shows the same information in its footer.
- The source link comes from the `SOURCE_URL` setting, which defaults to the canonical repository
  URL. Operators running a modified version MUST point it at their modified source. See
  [../operations/self-hosting.md](../operations/self-hosting.md).
- `/about` text goes through i18n like any other bot text, but the attribution line and the
  license name are never translated or removed.

## Source files and packages

> Status: `Proposed`

- Every source file starts with an SPDX header in the file type's comment syntax:
  `// SPDX-License-Identifier: AGPL-3.0-or-later` (Rust, WIT) or
  `# SPDX-License-Identifier: AGPL-3.0-or-later` (TOML, Fluent, shell). Files in the permissive
  paths use `MIT OR Apache-2.0`.
- Every crate declares its license in `Cargo.toml` (`license = "AGPL-3.0-or-later"` or
  `license = "MIT OR Apache-2.0"`).
- Every plugin manifest declares its license (see [../architecture/plugins.md](../architecture/plugins.md)).
- Dependencies MUST have licenses compatible with AGPL-3.0-or-later (e.g. MIT, BSD, ISC,
  Apache-2.0, Zlib, MPL-2.0, LGPL, GPL-3.0). Dependencies of the permissive parts MUST themselves be
  permissive. CI enforces both with cargo-deny.

## Contributions

> Status: `Proposed`

- Contributions are accepted under the license of the path they touch (inbound = outbound). There
  is no CLA.
