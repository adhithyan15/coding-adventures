# VLT-PM37 — Audited Authored Database-Credential Conflict Merge

## Status

Normative Phase 1A contract for resolving one current database-credential
conflict with a complete user-authored database-credential record.

## 1. Command and boundary

```text
vault-pm [--vault NAME] conflict merge database-credential ITEM BASE_REVISION
```

The exact current live database-credential base supplies immutable
identity/creation time and the favorite, collection, tag, and attachment state
not editable by the Phase 1A form. The controlling terminal collects a complete
label, engine, host, port line, optional database name, username, and hidden
password. Existing passwords are inspected only through `conflict reveal`; the
merge command never prefills candidate values, accepts inline fields, or
chooses a winner. Other schemas remain separate backlog ceremonies.

An authored merge result is a static credential: exactly as in
`VLT-PM28-cli-database-credential-create.md`, a locally typed credential has no
lease ID and no lease expiry, so the merged record carries neither. Carrying a
base candidate's lease forward would attach dynamic-issuance state to a
hand-typed secret that no issuer ever vouched for; VLT08 dynamic leases remain
out of scope.

## 2. Closed form validation

The host bounds every field before application entry. The application also
requires an engine of 1–32 ASCII bytes that starts with a lowercase letter and
continues only with lowercase letters, digits, `-`, or `_`, and a port that is
one canonical decimal integer in `1..=65535` with no sign and no leading zero.
This defense-in-depth validation occurs inside the opaque preparation so every
invalid complete form publishes its failed audit event before the closed error
returns. Phase 1A intentionally performs no network, DNS, driver, TLS,
connection, lease, or rotation lookup, and makes no claim that the engine,
endpoint, account, or password is valid or reachable.

## 3. Opaque preparation and audit ordering

Time and audit-failure randomness are reserved before authentication. The
application consumes the unlocked session and requires an active audit epoch,
at least two current candidates, exact current membership of `BASE_REVISION`,
a live item-bound database-credential base, and compatible
identity/schema/creation time across every retained live candidate.

A ready opaque preparation owns the complete wipe-on-drop base without
returning it to the CLI. Missing, unconflicted, noncurrent, tombstone,
cross-item, and wrong-schema bases publish failed item-scoped
`ItemConflictMerge` events before their closed error. Prompt, form-validation,
or mutation-entropy failure consumes the preparation and publishes the same
failure before the host error. Stale pins fail closed; ambiguous publication
retains the exact journal.

Success replaces the complete database-credential payload, preserves base
non-form metadata, names the entire former current set as direct causal
parents, and publishes a succeeded `ItemConflictMerge` atomically. Because the
result is authored, its event intentionally omits selected revision. Events
contain no base/candidate identity, label, engine, host, port, database name,
username, password, password length or prefix, lease detail, prompt progress,
provider detail, or arbitrary error.

## 4. Output and storage neutrality

Success emits only `Conflict merged: ITEM`; failure has empty stdout. The
password is a hidden terminal input and all authored fields stay in
wipe-on-drop ownership until sealed. No database-credential value enters
arguments, stdout/stderr, audit history, debug output, config, or durable
plaintext. Repository and local-state access remain injected and provider
neutral.

## 5. Acceptance gates

Tests must prove exact default/named grammar; audited missing, unconflicted,
noncurrent, tombstone, wrong-schema, prompt, validation, and entropy failures;
one all-current-parent success that preserves base metadata and immutable
history; restart-backed redacted observation; password exclusion; named-target
isolation; formatting, Clippy, rustdoc, application/CLI tests; and a real
executable PTY failure journey that stops before the authored form when the
target is not a conflict.
