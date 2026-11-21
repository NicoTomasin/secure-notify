# Sources catalog

Candidate sources for each category in [scope.md](scope.md). This is a working catalog, not a
commitment: every entry MUST be checked for terms of use, access method, rate limits, and data
quality before it is implemented. The project aims for as many sources as possible, using only
free access.

## Free access

> Status: `Decided`

- Built-in sources and exposure providers SHOULD use free access only: public pages (scraping),
  feeds, or free APIs. Free API keys are acceptable.
- Candidates are tried as long as possible and dropped when they turn out to require payment or
  another barrier that cannot be removed.

## Default enablement

> Status: `Decided`

- Coverage is "the more, the better": every built-in source that passes review is enabled by
  default (`default_enabled = true`).
- Admins can disable any source for their instance; subscribers can exclude sources with rules (see
  [../architecture/matching-and-rules.md](../architecture/matching-and-rules.md)).
- Noise is controlled by matching and rules, not by limiting sources: a story is only notified when
  it is relevant to a subscriber.

## Catalog rules

> Status: `Proposed`

- Sources that require an API key (even a free one) are disabled until the operator provides the
  key.
- Prefer structured data (APIs, data dumps, CSAF, OSV) over feeds, and feeds over scraping.
- Community and social sources get a low default severity and are never the sole basis for
  `critical`.
- Every source follows the acceptance criteria in
  [../community/contributing.md](../community/contributing.md) and the ethical boundaries in
  [vision.md](vision.md).

## Candidates

> Status: `Proposed`

| Category | Candidates | Likely runtime |
|---|---|---|
| Vulnerability databases | OSV (aggregates GitHub Advisory Database, PyPA, RustSec, Go vulnerability database, and other ecosystem databases), GitHub Advisory Database, NVD API, CVE Program records (cvelistV5), EU Vulnerability Database (EUVD) | native |
| Distribution trackers | Debian, Ubuntu (USN), Red Hat and other CSAF publishers, Alpine secdb | native or WASM |
| Exploitation signals | CISA KEV, FIRST EPSS (enricher), Exploit-DB, Metasploit module additions, nuclei-templates additions, tl;drCVE (tldrcve.com: RSS feed of curated, plain-English summaries of the most discussed CVEs, ranked by in-the-wild exploitation, KEV listing, and public exploit code) | native, WASM, or declarative |
| Malicious packages | OSV `MAL-` entries (OpenSSF malicious packages), registry security announcements | native |
| Vendor advisories | Vendor CSAF feeds and PSIRT RSS/APIs (e.g. Microsoft, Cisco, Fortinet, Palo Alto Networks, Ivanti, Citrix, VMware, Atlassian, Oracle, SAP, Siemens, Apple, Google Chrome, Mozilla) | declarative or WASM |
| National CERTs | CISA advisories (including ICS), CERT-EU, NCSC (UK), CERT-Bund (CSAF), CERT-FR, JPCERT, other national CERTs with RSS or CSAF | declarative |
| Breaches | Free public breach catalogs (e.g. the Have I Been Pwned breach list), official breach notification portals, data breach reporting sites | WASM or declarative |
| Attacks and ransomware | Public ransomware tracking aggregators, security news outlets (e.g. BleepingComputer, The Record, SecurityWeek, Krebs on Security, The Hacker News, Help Net Security) | declarative |
| Outages | Status pages with standard JSON APIs or feeds (e.g. GitHub, Cloudflare, Atlassian, Slack), cloud provider status feeds | declarative (`statuspage`, `rss`) |
| Security releases | Project security release feeds (e.g. Node.js, Bun, Python, PHP, Go, Rust, Kubernetes, OpenSSL, nginx, PostgreSQL), repository release feeds | declarative |
| Community signals | Security subreddits, Mastodon hashtags and instances with feeds | declarative |
| Exposure providers | Free breach lookup APIs for email addresses and domains (e.g. XposedOrNot); paid-only lookup APIs are excluded | native or WASM |
