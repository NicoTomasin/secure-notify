# Platforms

secure-notify talks to people through platform plugins: Telegram first, then other chat platforms
and delivery-only channels such as email and webhooks. A platform-agnostic **conversation layer**
in the core defines commands, permissions, and messages once; each platform plugin renders them
in its platform's native form and reports what it supports through capability flags.

## Requirements

> Status: `Decided`

- secure-notify supports multiple platforms; one subscriber can link endpoints on several of them
  (see [subscribers.md](subscribers.md)).
- Telegram comes first because it is free to use for bots (full order below).

## Platform plugins

> Status: `Proposed`

- Every platform plugin MUST implement **delivery**: send a rendered message (single story,
  digest, command reply) to an endpoint.
- A platform plugin MAY implement **interaction**: receive commands, button actions, and file
  uploads, and report endpoint lifecycle events (bot added to or removed from a group, user blocked
  the bot).
- Platform plugins are native (see [plugin-runtimes.md](plugin-runtimes.md)) and MUST connect
  using outbound connections only (long polling, WebSockets, or outbound HTTP), so instances need
  no inbound exposure (see [../operations/self-hosting.md](../operations/self-hosting.md)).
  Platforms that can only work with inbound webhooks are supported only through the optional HTTP
  server.

### Capabilities

> Status: `Proposed`

Each plugin declares capabilities the core uses to adapt messages:

| Capability | Examples of effect |
|---|---|
| `interactive` | Commands available; otherwise configuration happens elsewhere. |
| `buttons` | Inline actions rendered as buttons; otherwise as command hints or panel links. |
| `uploads` | Files accepted for parsing; maximum file size. |
| `groups` | Endpoint kinds `group`/`channel` available for teams. |
| `formatting` | Rich text model (HTML subset, Markdown dialect, blocks, plain text). |
| `max_message_length` | Splitting threshold for long messages and digests. |
| `threads` | Digests and update notices can reply in a thread. |

## Conversation layer

> Status: `Proposed`

- Commands are defined once in the core: name, arguments, required permission (any member,
  manager, admin), availability (private, group), and localized help texts.
- Platform plugins render commands natively: Telegram bot commands, Discord and Slack slash
  commands, Matrix and Mattermost text commands with a prefix (e.g. `!sn watch php`).
- Messages are built in a platform-neutral model (header, severity badge, fields, lists, links,
  actions). Each plugin renders the model for its platform.
- All content from sources (titles, summaries, URLs) MUST be escaped for the target format, and only
  `http`/`https` links are rendered (see [security.md](security.md)).

## Platform order

> Status: `Decided`

1. Telegram.
2. Email and webhook.
3. Slack, Discord, and WhatsApp.

Further platforms come after these, in an order decided later.

## Operator onboarding

> Status: `Decided`

When a platform's bot API requires onboarding steps (registering a phone number, a business
account, an app in a workspace) or charges for messages, the integration is still provided: the
operator completes the onboarding with their own accounts and bears any costs. Each platform's
documentation states those steps and costs.

## Planned platforms

> Status: `Proposed`

| Platform | Mode | Connection | Notes |
|---|---|---|---|
| Telegram | interactive | Bot API long polling | Private chats and groups; 4096-character messages; uploads via the Bot API; privacy mode in groups requires uploads as replies to a bot prompt. |
| Email | delivery-only | SMTP (outbound) | Notifications, digests, and identity verification codes; multipart HTML and plain text. |
| Webhook | delivery-only | Outbound HTTP POST | JSON payload with the message model and story data, signed with HMAC-SHA256; for custom integrations. |
| Slack | interactive | Socket Mode WebSocket | Channels and DMs; slash commands and Block Kit; each operator creates their own Slack app. |
| Discord | interactive | Gateway WebSocket | DMs and guild channels; slash commands and buttons; managers are members with channel-management permission. |
| WhatsApp | interactive with the HTTP server, otherwise delivery-only | Official WhatsApp Business Platform (Cloud API): outbound HTTP to send; inbound webhook to receive | The operator registers their own phone number and business account with Meta and pays Meta's charges for business-initiated (template) messages. Receiving commands requires the optional HTTP server with a public HTTPS URL (see [web-panel.md](web-panel.md)); without it, WhatsApp endpoints are linked from another platform and only receive notifications. |
| Matrix | interactive | Client-server sync | Later. Rooms and direct chats; text commands; unencrypted rooms first. |
| Mattermost | interactive | WebSocket API | Later. Channels and DMs; text commands. |
| ntfy, Gotify | delivery-only | Outbound HTTP | Later. Push notifications; priority mapped from severity. |
| Signal | interactive (optional) | Through a separately run Signal REST bridge | Later. Not part of the single-container deployment; depends on an unofficial client. |

## Command set

> Status: `Proposed`

| Command | Who | Purpose |
|---|---|---|
| `/start [code]` | anyone | Pair (first admin), join with an invite, or join in open registration mode. |
| `/help` | subscriber | List commands. |
| `/about` | subscriber | Attribution, license, version, source link (see [../product/licensing.md](../product/licensing.md)). |
| `/link`, `/unlink` | subscriber (manager in teams) | Link and unlink endpoints (see [subscribers.md](subscribers.md)). |
| `/watch`, `/unwatch`, `/watching` | subscriber | Manage watched entities (see [taxonomy.md](taxonomy.md)). |
| `/upload` | subscriber | Prompt for a file (required in groups on some platforms). |
| `/inventory` | subscriber | List, add, remove, or clear inventory entries (see [inventory.md](inventory.md)). |
| `/identities` | subscriber | Manage watched identities (see [exposure-monitoring.md](exposure-monitoring.md)). |
| `/rules` | subscriber | View, edit, and reset rules (see [matching-and-rules.md](matching-and-rules.md)). |
| `/delivery` | subscriber | View and edit delivery modes and routing (see [delivery.md](delivery.md)). |
| `/mutes` | subscriber | List and remove mutes. |
| `/sources` | subscriber | List enabled sources; exclude one (creates an `ignore` rule). |
| `/search <query>` | subscriber | Search stored stories by identifier, entity, or text. |
| `/report` | subscriber | One-off report of everything currently known to affect the subscriber's inventory and watches. |
| `/panel` | subscriber | Get a one-time login link for the web panel, when enabled (see [web-panel.md](web-panel.md)). |
| `/timezone`, `/language` | subscriber | Set timezone and bot language. |
| `/invite`, `/subscribers`, `/remove`, `/registration` | admin | Manage access (see [subscribers.md](subscribers.md)). |
| `/status` | admin | Source, plugin, and content health; pending approvals; instance stats. |

In groups, commands that change settings are limited to managers.
