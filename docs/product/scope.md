# Scope

What secure-notify covers: the kinds of happenings it reports (event kinds) and the things a
subscriber can watch (watch targets). This doc is the single definition of both lists; the data
model, matching, and delivery docs link here instead of restating them.

## Coverage

> Status: `Decided`

secure-notify aims to cover every category of security and reliability information that affects a
technical user's software, infrastructure, services, or identities, as completely as the available
public sources allow.

## Event kinds

> Status: `Proposed`

Every event and story has exactly one kind.

| Kind | Meaning | Examples | Family |
|---|---|---|---|
| `vulnerability` | A security flaw in software or hardware. | CVE in OpenSSL, GHSA in an npm package, vendor advisory for a firewall. | vulnerability |
| `exploit` | A public exploit or proof of concept, or a report of active exploitation, for a vulnerability. | Exploit-DB entry, new Metasploit module, KEV addition. | vulnerability |
| `malicious-package` | A package published with malicious code. | Typosquatted package, compromised maintainer account, self-propagating worm in a registry. | vulnerability |
| `release` | A security or breaking release announcement. | "Security releases scheduled for next Tuesday", patch release fixing CVEs. | vulnerability |
| `bug` | A critical non-security defect. | Data corruption bug in a database release. | vulnerability |
| `breach` | A confirmed or credibly reported data breach or leak at an organization. | "Example Corp leaked 2M customer records". | incident |
| `attack` | A security incident without confirmed data exposure. | Ransomware attack, infrastructure compromise, large DDoS campaign, CI/CD compromise. | incident |
| `outage` | A service disruption. | Status page incident at a cloud provider or SaaS. | incident |
| `news` | Other relevant security information with no more specific kind. | Analysis article, policy change, threat report. | both |

Families matter for correlation (see [../architecture/deduplication.md](../architecture/deduplication.md)):
events of different families are never merged into one story.

## Watch targets

> Status: `Decided` (target types) / `Proposed` (commands and relevance mapping)

| Target | Declared through | Produces relevance | Doc |
|---|---|---|---|
| Component (package by purl, product by CPE, optional exact version) | Uploaded manifests, lock files, and SBOMs; `/inventory add` | `exact-version`, `component` | [../architecture/inventory.md](../architecture/inventory.md) |
| Product or vendor | `/watch` | `product` | [../architecture/taxonomy.md](../architecture/taxonomy.md) |
| Organization (company or online service) | `/watch` | `organization` | [../architecture/taxonomy.md](../architecture/taxonomy.md) |
| Technology | `/watch`; derived automatically from uploads | `technology` | [../architecture/taxonomy.md](../architecture/taxonomy.md) |
| Watched identity (email address or domain) | `/identities add`, after ownership verification | `identity` | [../architecture/exposure-monitoring.md](../architecture/exposure-monitoring.md) |

How relevance levels are computed and ranked: [../architecture/matching-and-rules.md](../architecture/matching-and-rules.md).

## Source categories

> Status: `Proposed`

To reach the coverage goal, sources are needed in each of these categories. Candidate sources are
listed in [sources-catalog.md](sources-catalog.md).

- Vulnerability databases (global, ecosystem-specific, distribution trackers).
- Exploitation signals (known-exploited catalogs, exploit prediction scores, public exploits).
- Malicious package reports.
- Vendor security advisories.
- National CERT advisories.
- Breach and attack reporting (breach catalogs, ransomware trackers, breach notification portals,
  security news).
- Service status pages.
- Project security release feeds.
- Community signals (low default severity, never sole basis for `critical`).
