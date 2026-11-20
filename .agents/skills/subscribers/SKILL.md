---
name: subscribers
description: Change how people and teams join and are managed in a secure-notify instance. Covers the pairing code, invites, open registration and its limits, endpoints and linking codes, the primary endpoint, admin and member roles, group manager permissions, and deactivation when the bot loses access. Use for subscriber and access code in crates/core and the related commands.
---

# Subscribers and access

**Use when:** changing pairing, invites, registration, endpoint linking, roles, manager checks, or
deactivation.
**Not for:** web panel login sessions ([web-panel](../web-panel/SKILL.md)); the command plumbing
itself ([bot-command](../bot-command/SKILL.md)).

## Read first

- [subscribers.md](../../../docs/architecture/subscribers.md) (all sections)
- [security.md: Threats and mitigations](../../../docs/architecture/security.md#threats-and-mitigations)
  (abusive users, impersonation in groups)
- [data-model.md: Persistence entities](../../../docs/architecture/data-model.md#persistence-entities)
  (`subscribers`, `endpoints`, `link_codes`, `invites`, `registration_requests`)

## Procedure

1. **Codes** (pairing, invite, link) are single-use and expire as documented; generate them with a
   cryptographically secure random source.
2. **Unknown chats are ignored,** except `/start` with a valid code, or `/start` alone in open
   registration mode; they get no data and run no command.
3. **Managers** are the platform-level administrators of a team's group endpoints, checked through
   the platform API at command time.
4. **Endpoints:** exactly one primary endpoint per subscriber; the last interactive endpoint cannot
   be unlinked; losing access deactivates the endpoint, and a subscriber with no active endpoint is
   deactivated with its queue dropped and its configuration kept.
5. **Open registration** respects every limit (`max_subscribers`, platforms, teams, approval, rate
   limit) and is off by default.
6. **Onboarding message** states what is stored, per
   [Roles and permissions](../../../docs/architecture/subscribers.md#roles-and-permissions).
7. **Tests:** code expiry and reuse; a non-manager denied in a team; the last interactive endpoint
   protected; open registration caps and approval; messages from unregistered chats produce no reply.
   Load [security-sensitive-change](../security-sensitive-change/SKILL.md) for the adversarial cases.

## Verify

In `dev console`, acting as several users (`:as`): pair the admin with `/start {pairing_code}`;
create an invite and join as a second user with `{invite_code}`; try the code again (reuse) and
after `:advance` past its expiry; link a second endpoint with `/link` and `{link_code}`; try admin
commands as a member; enable open registration with a cap of one and exceed it.

## Done when

- [ ] Code handling, unknown-chat handling, and permission checks are tested, including abuse cases.
- [ ] Pairing, invite, linking, and registration flows were verified locally.
