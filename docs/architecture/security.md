# Security

secure-notify is a security tool, so its own security is part of the product. This doc is the
threat model of an instance and of the project's supply chain: what is protected, from whom, and
how. Rules about specific data (inventory, watched identities) live in their own docs and are
linked from here.

## Assets

> Status: `Proposed`

| Asset | Where | Doc |
|---|---|---|
| Inventories (component versions per subscriber) | Database | [inventory.md](inventory.md) |
| Watched identities | Database, encrypted | [exposure-monitoring.md](exposure-monitoring.md) |
| Endpoints (chat IDs, email addresses, webhook URLs) | Database | [subscribers.md](subscribers.md) |
| Platform tokens, provider API keys, encryption key | Operator configuration | [../operations/self-hosting.md](../operations/self-hosting.md) |
| Content signing key | Project CI | [content-registry.md](content-registry.md) |
| Release artifacts | Project releases | [../engineering/releases.md](../engineering/releases.md) |

## Threats and mitigations

> Status: `Proposed`

| Threat | Mitigations |
|---|---|
| Hostile fetched content (malformed feeds, huge responses, injected markup, misleading links) | Size and time limits; streaming parsers; schema validation; output escaping per platform; only `http`/`https` links; no content-based code execution. |
| Hostile uploads (zip bombs, deeply nested documents, billion-laughs, oversized files) | Size and complexity limits enforced before parsing; safe parsers; parsing off the async executor with timeouts; parsers have no network, filesystem, or state. |
| Malicious or compromised plugins | WASM sandbox with memory, CPU, and output limits; capabilities (hosts, secrets, state) declared and enforced; capability expansions need admin approval; external plugins need explicit enablement. |
| SSRF through sources, redirects, or federation URLs | The shared HTTP client refuses private, loopback, link-local, and metadata addresses (also after redirects and DNS resolution) unless allowed by the operator; plugins can reach only declared hosts. |
| Compromised content registry or CI | Signed content bundles verified against keys embedded in the binary; protected release environment; pinned CI actions; signed releases. |
| Abusive users on open-registration instances | Registration limits and approval; per-subscriber command and upload rate limits; inventory and watch size caps; admins can remove subscribers. |
| Impersonation in groups | Manager permissions checked through the platform API at command time; unknown chats ignored. |
| Curious operator or database leak | Data minimization (anonymous inventory, no files); watched identities encrypted and hashed; no personal data in logs. |
| Network attacker | TLS for every outbound connection; certificate validation always on; web panel served behind a TLS reverse proxy. |
| Web panel attacks (CSRF, XSS, session theft) | See [web-panel.md](web-panel.md). |

## Secrets

> Status: `Proposed`

- Secrets come from environment variables or secret files, never from the database or the instance
  config file. They MUST NOT appear in logs, error messages, or panel pages.
- The encryption key for watched identities protects database files that leak on their own
  (backups, copies). It does not protect against an attacker who controls the running host,
  because the instance must be able to decrypt identities to check them. Docs and the onboarding
  message state this honestly.

## Logging

> Status: `Proposed`

Logs are structured and MUST NOT contain uploaded file contents, inventory entries, watched
identities, message bodies sent to subscribers, tokens, or API keys. Endpoint addresses are logged
only as internal IDs.

## Supply chain

> Status: `Proposed`

- Dependencies are checked in CI with cargo-deny (licenses, known advisories, banned crates,
  allowed registries). New dependencies need a justification in the pull request.
- CI actions are pinned by commit SHA; CI tokens have minimal permissions.
- Releases are built in CI, signed, and published with an SBOM and build provenance (see
  [../engineering/releases.md](../engineering/releases.md)).
- **Dogfooding:** the project runs secure-notify against its own `Cargo.lock` and content, so it is
  notified about its own vulnerable dependencies.

## Vulnerability disclosure

> Status: `Proposed`

Vulnerabilities in secure-notify are reported privately as described in
[`../../SECURITY.md`](../../SECURITY.md).
