# SPDX-License-Identifier: AGPL-3.0-or-later
# English: the reference locale (docs/architecture/delivery.md#localization-i18n).

language-name = English

## Command descriptions shown by /help

command-start = Join this instance with a code
command-help = List the commands you can use
command-about = License, version, and source code
command-upload = How to send a manifest or lock file
command-inventory = List, add, or remove inventory entries
command-watch = Watch a technology
command-unwatch = Stop watching something
command-watching = List what you watch
command-rules = Show your rules and set a minimum severity
command-language = Show or change the bot language
command-invite = Create an invite code for a person (admins)
command-subscribers = List subscribers (admins)
command-remove = Remove a subscriber and their data (admins)

## /start

start-admin = You are now the admin of this instance.
start-member = Welcome! You joined this instance.
start-already-registered = You are already registered. Send /help to see the commands.
onboarding =
    What this bot stores about you: your chat ID on this platform, your language, what you watch, and the inventory you upload or add, reduced to component names and versions. Uploaded files are never stored, and the inventory is not grouped by project.
    The operator of this instance can read that data. Send /help to get started.

## /help

help-header = Commands you can use:
help-line = /{ $command } — { $description }

## /about

about-version = Version: { $version }
about-license = License: { $license }
about-source = Source code: { $url }

## /upload and uploads

upload-prompt = Send one of these files in this chat: { $files }. It is read once and never stored; only component names and versions are kept.
upload-done = Inventory updated from { $file }: { $added } new, { $refreshed } already known.
upload-technologies = Technologies detected, now watched: { $technologies }.
upload-unsupported = I cannot read { $file }. Supported files: { $files }.
upload-too-large = The file is larger than { $limit } MB.
upload-too-many = The file lists more than { $limit } components.
upload-timeout = Reading the file took too long. Try a smaller file.
upload-malformed = { $file } could not be read; it does not look like a valid file of its kind.
upload-bun-binary = Bun’s binary lockfile (bun.lockb) is not supported. Bun 1.2 and later write the text lockfile bun.lock; with older versions run “bun install --save-text-lockfile”. Then send bun.lock.

## /inventory

inventory-empty = Your inventory is empty. Send a lock file or use /inventory add <purl> <version>.
inventory-header = { $count ->
    [one] One entry:
   *[other] { $count } entries:
}
inventory-no-version = (no version)
inventory-more = …and { $count } more.
inventory-usage = Use /inventory add <purl> [version], /inventory remove <purl>[@version], or /inventory clear.
inventory-added = Added { $component } { $version }.
inventory-invalid = “{ $component }” is not a package URL such as pkg:npm/lodash.
inventory-removed = { $count ->
    [one] Removed one entry.
   *[other] Removed { $count } entries.
}
inventory-cleared = Your inventory was cleared ({ $count } entries removed).

## /watch, /unwatch, /watching

watch-usage = Usage: /watch <name>, for example /watch Node.js. You can watch: { $available }.
watch-added = You now watch { $name }.
watch-already = You already watch { $name }.
watch-unknown = “{ $query }” is not in the catalog. You can watch: { $available }.
unwatch-usage = Usage: /unwatch <name>, as listed by /watching.
unwatch-removed = You no longer watch { $name }.
unwatch-not-watched = You do not watch { $name }.
watching-none = You do not watch anything yet. Use /watch <name>.
watching-header = You watch:
watching-line = { $name } ({ $id }){ $origin ->
    [derived] {" "}· from your uploads
   *[manual] {""}
}

## /rules

rules-header = { $default ->
    [yes] Your rules (the defaults), in order; the first that matches decides:
   *[no] Your rules, in order; the first that matches decides:
}
rule-notify = { $position }. Notify when { $conditions }.
rule-notify-min = { $position }. Notify when { $conditions }, if the severity is { $level } or higher.
rule-ignore = { $position }. Ignore when { $conditions }.
rules-min-severity = Minimum severity: { $level }. Exploited stories are always notified.
rules-min-severity-none = No minimum severity is set.
rules-usage = Use /rules severity <critical|high|medium|low|off> or /rules reset.
rules-severity-set = Minimum severity set to { $level }. Exploited stories are still notified.
rules-severity-off = Minimum severity removed.
rules-severity-invalid = “{ $level }” is not a level. Use critical, high, medium, low, or off.
rules-reset = Your rules were reset to the defaults.

## /language

language-current = Current language: { $language }. Available: { $available }.
language-usage = Change it with /language <code>, for example /language es.
language-changed = Language changed to { $language }.
language-unknown = “{ $code }” is not an available language. Available: { $available }.

## /invite

invite-created = Invite code: { $code }
invite-instructions = The person sends /start { $code } to this bot in a private chat. The code works once and expires on { $expires }.

## /subscribers

subscribers-header = { $count ->
    [one] One subscriber:
   *[other] { $count } subscribers:
}
subscribers-line = #{ $id } · { $role } · { $language } · since { $since }{ $active ->
    [no] {" "}· inactive
   *[yes] {""}
}
role-admin = admin
role-member = member

## /remove

remove-usage = Usage: /remove <subscriber number>, as listed by /subscribers.
remove-done = Subscriber #{ $id } and their data were removed.
remove-not-found = There is no subscriber #{ $id }.
remove-self = You cannot remove yourself.

## Errors

error-permission-denied = Only admins can use /{ $command }.
error-unknown-command = Unknown command. Send /help to see the commands.
error-internal = Something went wrong. Please try again later.

## Notifications

severity-critical = CRITICAL
severity-high = HIGH
severity-medium = MEDIUM
severity-low = LOW
severity-unknown = UNKNOWN SEVERITY
kind-vulnerability = Vulnerability
kind-exploit = Exploit
kind-malicious-package = Malicious package
kind-release = Security release
kind-bug = Critical bug
kind-breach = Breach
kind-attack = Attack
kind-outage = Outage
kind-news = News
notice-header = [{ $severity }] { $kind }: { $title }
notice-update-header = Update · [{ $severity }] { $kind }: { $title }
notice-exploited = Known to be exploited.
notice-exploit-available = A public exploit is available.
change-severity = Severity raised to { $severity }.
change-exploited = It is now known to be exploited.
change-exploit-available = A public exploit is now available.
change-fix-available = A fix is now available.
reason-version-fixed = In your inventory: { $component } { $version } (fixed in { $fixed }).
reason-version = In your inventory: { $component } { $version }.
reason-component = In your inventory: { $component } (version unknown).
reason-watch = You watch { $name }.
field-cvss = CVSS
field-epss = EPSS
notice-sources = { $count ->
    [one] Reported by one source.
   *[other] Reported by { $count } sources.
}
notice-link = Details
action-mute-story = Mute this story
action-mute-component = Mute { $component }
action-no-longer-used = I no longer use { $component } { $version }
action-mute-entity = Mute { $name }

## /mutes

command-mutes = List, add, and remove mutes
mutes-none = You have no mutes.
mutes-header = Your mutes:
mutes-line = #{ $id } · { $scope } { $value }
mutes-line-until = #{ $id } · { $scope } { $value } until { $until }
mutes-usage = Use /mutes add story <id>, entity <id>, component <purl>, or version <purl@version>, with an optional duration such as 7d; /mutes remove <number>.
mutes-added = Muted { $scope } { $value }.
mutes-added-until = Muted { $scope } { $value } until { $until }.
mutes-invalid = That mute is not valid.
mutes-removed = Mute #{ $id } removed.
mutes-not-found = There is no mute #{ $id }.
action-details = Details
details-gone = That story is no longer available.
details-identifiers = Identifiers: { $ids }
details-sources = Sources: { $sources }
details-affected = Affected: { $component } (fixed in { $fixed })
details-affected-unfixed = Affected: { $component } (no fix yet)
details-related = Related: { $title }

## Initial scan

upload-scan-deferred = Vulnerability data is still synchronizing for the first time. The initial scan report will follow when it completes.
scan-none = Initial scan of { $entries ->
    [one] one entry
   *[other] { $entries } entries
}: no known issues that were not already reported to you.
scan-header = Initial scan of { $entries ->
    [one] one entry
   *[other] { $entries } entries
}: { $count ->
    [one] one known issue not reported to you before.
   *[other] { $count } known issues not reported to you before.
}
scan-story = [{ $severity }] { $title }: { $entries }
scan-story-exploited = [{ $severity }, exploited] { $title }: { $entries }
scan-entry-fixed = { $component } { $version }, fixed in { $fixed }
scan-entry = { $component } { $version }, no fix yet
scan-entry-unknown = { $component } (version unknown)
