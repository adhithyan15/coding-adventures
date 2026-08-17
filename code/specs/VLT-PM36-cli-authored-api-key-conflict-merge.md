# VLT-PM36 — Audited Authored API-Key Conflict Merge

## Status

Normative Phase 1A contract for resolving one current API-key conflict with a
complete user-authored API-key record.

## 1. Command and boundary

```text
vault-pm [--vault NAME] conflict merge api-key ITEM BASE_REVISION
```

The exact current live API-key base supplies immutable identity/creation time
and the favorite, collection, tag, and attachment state not editable by the
Phase 1A form. The controlling terminal collects a complete label, service,
hidden token, an optional comma-separated scope line, and an optional expiry in
Unix seconds. Existing tokens are inspected only through `conflict reveal`; the
merge command never prefills candidate values, accepts inline fields, or
chooses a winner. Other schemas remain separate backlog ceremonies.

## 2. Closed form validation

The host bounds every field before application entry. The application also
requires a scope line of at most 2,048 UTF-8 bytes that, when non-empty, splits
only on commas into at most 64 already-trimmed, non-empty, unique components of
at most 256 UTF-8 bytes each in their original order, and an expiry that is
either empty or one canonical nonzero unsigned decimal integer with no sign and
no leading zero. This defense-in-depth validation occurs inside the opaque
preparation so every invalid complete form publishes its failed audit event
before the closed error returns. Phase 1A intentionally performs no network,
issuer, token-verification, revocation, rotation, or scope lookup.

## 3. Opaque preparation and audit ordering

Time and audit-failure randomness are reserved before authentication. The
application consumes the unlocked session and requires an active audit epoch,
at least two current candidates, exact current membership of `BASE_REVISION`,
a live item-bound API-key base, and compatible identity/schema/creation time
across every retained live candidate.

A ready opaque preparation owns the complete wipe-on-drop base without
returning it to the CLI. Missing, unconflicted, noncurrent, tombstone,
cross-item, and wrong-schema bases publish failed item-scoped
`ItemConflictMerge` events before their closed error. Prompt, form-validation,
or mutation-entropy failure consumes the preparation and publishes the same
failure before the host error. Stale pins fail closed; ambiguous publication
retains the exact journal.

Success replaces the complete API-key payload, preserves base non-form
metadata, names the entire former current set as direct causal parents, and
publishes a succeeded `ItemConflictMerge` atomically. Because the result is
authored, its event intentionally omits selected revision. Events contain no
base/candidate identity, API-key fields, token length or prefix, scope, expiry,
prompt progress, provider detail, or arbitrary error.

## 4. Output and storage neutrality

Success emits only `Conflict merged: ITEM`; failure has empty stdout. The token
is a hidden terminal input and all authored fields stay in wipe-on-drop
ownership until sealed. No API-key value enters arguments, stdout/stderr, audit
history, debug output, config, or durable plaintext. Repository and local-state
access remain injected and provider neutral.

## 5. Acceptance gates

Tests must prove exact default/named grammar; audited missing, unconflicted,
noncurrent, tombstone, wrong-schema, prompt, validation, and entropy failures;
one all-current-parent success that preserves base metadata and immutable
history; restart-backed redacted observation; token exclusion; named-target
isolation; formatting, Clippy, rustdoc, application/CLI tests; and a real
executable PTY failure journey that stops before the authored form when the
target is not a conflict.
