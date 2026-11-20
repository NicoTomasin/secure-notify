# Security policy

## Reporting a vulnerability

Please do not open a public issue for security problems.

Report vulnerabilities privately through the repository's **private vulnerability reporting**
("Report a vulnerability" in the Security tab). Include a description, affected versions or
commits, reproduction steps, and the impact you expect.

The maintainers aim to acknowledge reports within 7 days, agree on a disclosure timeline with the
reporter, and publish a security advisory with the fix.

## Scope

In scope:

- the secure-notify code in this repository, including built-in plugins;
- content bundles and their signing and update process;
- release artifacts and container images.

Out of scope:

- incorrect or missing data from third-party sources (report it to the source);
- vulnerabilities in third-party plugins not maintained in this repository;
- instances misconfigured against the documentation (e.g. an exposed HTTP server without a TLS
  reverse proxy).

## Supported versions

There are no releases yet. Once releases exist, security fixes target the latest release.

The project's threat model is documented in
[docs/architecture/security.md](docs/architecture/security.md).
