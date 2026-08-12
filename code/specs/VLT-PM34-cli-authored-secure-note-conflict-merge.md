# VLT-PM34 — Audited Authored Secure-Note Conflict Merge

## Status

Normative Phase 1A contract for resolving one current secure-note conflict with
a complete user-authored note.

## 1. Command and boundary

```text
vault-pm [--vault NAME] conflict merge secure-note ITEM BASE_REVISION
```

The exact current live secure-note base supplies immutable identity/creation
time and the favorite, collection, tag, and attachment state not editable by
the Phase 1A form. The controlling terminal collects a complete title and
hidden body. Existing bodies are inspected only through `conflict reveal`; the
merge command never prefills a candidate, accepts inline fields, or chooses a
winner. Non-secure-note schemas remain separate backlog ceremonies.

## 2. Opaque preparation and audit ordering

Time and audit-failure randomness are reserved before authentication. The
application consumes the unlocked session and requires an active audit epoch,
at least two current candidates, exact current membership of `BASE_REVISION`,
a live item-bound secure-note base, and compatible identity/schema/creation
time across every retained live candidate.

A ready opaque preparation owns the complete wipe-on-drop base without
returning it to the CLI. Missing, unconflicted, noncurrent, tombstone,
cross-item, and wrong-schema bases publish failed item-scoped
`ItemConflictMerge` events before their closed error. Prompt or mutation-entropy
failure consumes the preparation and publishes the same failure before the host
error. Stale pins fail closed; ambiguous publication retains the exact journal.

Success replaces the complete note payload, preserves base non-form metadata,
names the entire former current set as direct causal parents, and publishes a
succeeded `ItemConflictMerge` atomically. Because the result is authored, its
event intentionally omits selected revision. Events contain no base/candidate
identity, title, body, prompt progress, provider detail, or arbitrary error.

## 3. Output and storage neutrality

Success emits only `Conflict merged: ITEM`; failure has empty stdout. The body
is hidden terminal input and stays in wipe-on-drop ownership. No body enters
arguments, stdout/stderr, audit history, debug output, config, or durable
plaintext. Repository and local-state access remain injected and provider
neutral.

## 4. Acceptance gates

Tests must prove exact default/named grammar; audited missing, unconflicted,
noncurrent, tombstone, wrong-schema, prompt, and entropy failures; one
all-current-parent success that preserves base metadata and immutable history;
restart-backed redacted observation; secret exclusion; named-target isolation;
formatting, Clippy, rustdoc, application/CLI tests; and a real executable PTY
failure journey that stops before the authored form when the target is not a
conflict.
