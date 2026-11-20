---
name: deployment
description: Change how operators get and run secure-notify. Covers the container image, the docker-compose example, release binaries and targets, startup, upgrade and backup behavior, and the self-hosting docs, while keeping the one-binary, one-volume, outbound-only deployment shape. Use when touching Dockerfile, compose files, release artifact definitions, or operator-facing setup.
---

# Deployment and packaging

**Use when:** changing the `Dockerfile`, the compose example, release targets, startup or upgrade
behavior, or [self-hosting.md](../../../docs/operations/self-hosting.md).
**Not for:** CI mechanics ([ci](../ci/SKILL.md)) or a new setting ([config-and-secrets](../config-and-secrets/SKILL.md)).

## Read first

- [self-hosting.md](../../../docs/operations/self-hosting.md) (all sections)
- [releases.md: Artifacts](../../../docs/engineering/releases.md#artifacts) and
  [Integrity](../../../docs/engineering/releases.md#integrity)
- [performance.md: Target hardware](../../../docs/architecture/performance.md#target-hardware)
- [web-panel.md: HTTP server](../../../docs/architecture/web-panel.md#http-server)

## Procedure

1. **Keep the deployment shape**
   ([Deployment shape](../../../docs/operations/self-hosting.md#deployment-shape)): one binary or
   one container, one data volume, one SQLite file, outbound connections only by default, no extra
   services.
2. **Image:** static musl binary, multi-arch, non-root user, read-only root filesystem, a single
   data volume ([Artifacts](../../../docs/engineering/releases.md#artifacts)). Prefer a minimal
   base image without build tools.
3. **Compose example:** one service, one volume, secrets through environment or secret files, no
   published ports by default.
4. **Startup and upgrades:** migrations run automatically after the automatic backup, each in its
   own transaction ([migrations.md](../../../docs/engineering/migrations.md)); check that an upgrade
   on an existing volume keeps working after your change.
5. **Operator docs** change in the same pull request: first run, configuration, backups, upgrades.
   Platform onboarding steps and costs live with the platform
   ([platforms.md: Operator onboarding](../../../docs/architecture/platforms.md#operator-onboarding)).
6. **Resource use:** idle memory and binary size stay within budget.

## Verify

1. Build the image locally.
2. Run it with a fresh volume and a test bot token: the pairing code appears in the logs; pairing
   works.
3. Restart: state persists.
4. Upgrade: start the previous image on a volume, then the new image on the same volume; check the
   backup file, the migration, and a working flow.
5. Confirm the container runs as non-root with a read-only root filesystem and no published ports,
   and measure idle memory.

## Done when

- [ ] The deployment shape is unchanged or the change was decided in the docs first.
- [ ] Fresh install, restart, and upgrade were verified locally.
- [ ] Operator docs match the new behavior.
