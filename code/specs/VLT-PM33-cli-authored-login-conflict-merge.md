# VLT-PM33 — Audited Authored Login Conflict Merge

## Status

Normative Phase 1A contract for resolving one current login conflict with a
complete user-authored login document.

## 1. Purpose and boundary

The local CLI exposes:

```text
vault-pm [--vault NAME] conflict merge login ITEM BASE_REVISION
```

`BASE_REVISION` is one exact current live login candidate. It supplies only
the immutable item identity and creation time plus the existing favorite,
collection, tag, and attachment state that the Phase 1A form cannot yet edit.
The user authors the complete login payload through the same bounded terminal
form as login create/edit. The command does not silently merge fields, choose
a candidate payload, accept secrets in arguments, reveal existing secrets,
discard immutable history, or support non-login schemas.

Payment-card, secure-note, API-key, database-credential, TOTP, and opaque-record
authored merge ceremonies remain explicit backlog items.

## 2. Closed grammar and form

`ITEM` and `BASE_REVISION` use canonical uppercase user encodings. Missing,
extra, lowercase/noncanonical identity, unknown record-kind, flag, stdin,
environment, and inline-field forms fail before authentication.

After authentication and application-owned base validation, the controlling
terminal collects the complete login title, username, zero-to-sixteen ordered
URLs, hidden password, and optional hidden notes. Existing candidate secrets
are never prefilled or returned to the host; users inspect them only through
the separately authorized `conflict reveal` ceremony.

## 3. Opaque application preparation

The application consumes the unlocked session and validates that:

1. `ITEM` exists with at least two current candidates;
2. `BASE_REVISION` belongs to that exact current set;
3. the base is live, belongs to `ITEM`, and has the login schema; and
4. the complete current set remains eligible for the existing authored-merge
   primitive, including compatible schema and creation time across live
   candidates and at least one live candidate.

Success returns an opaque, non-cloneable, non-debug preparation that owns the
session, base revision, and wipe-on-drop base document. Neither a complete
candidate document nor a revision-disclosure shortcut crosses into the CLI.

Completion replaces the entire login payload while retaining the base's
non-form metadata. The new revision names every exact current candidate as a
direct causal parent. Every old candidate and its ancestry remain immutable.

## 4. Audit-first ordering

The command requires an active audit epoch. Advisory time and failure-audit
randomness are reserved before authentication. After a ready preparation,
prompt, terminal, entropy, and form-validation failures publish before their
closed outcome becomes observable. Stale pins fail closed without mutation,
while an ambiguous repository or final local publication retains the exact
recovery journal.

Missing item, unconflicted item, wrong/noncurrent base, tombstone base,
cross-item identity, non-login base, incompatible live candidates, prompt
failure, and invalid authored form publish item-scoped failed
`ItemConflictMerge` events. Successful merge publishes the succeeded event
atomically with the all-current-parent revision. Authored merges do not select
one winning revision, so their events intentionally omit selected revision.

Audit events contain no base identity, candidate identity, schema, title,
username, URL, password, notes, form-progress, provider detail, or arbitrary
error text.

## 5. Output and storage neutrality

Success emits only:

```text
Conflict merged: ITEM
```

Every failure has empty stdout. The command routes through the injected
application repository and local-state store; it derives no provider address
or path. Plaintext form values are owned by wipe-on-drop containers and never
enter arguments, config, audit history, `Debug`, or durable local/provider
state.

## 6. Stable failures

Wrong passphrase returns locked. Missing item or noncurrent base returns
not-found after a durable failed event. An unconflicted item returns
conflict-required. Tombstone, identity, or authored-form mismatch returns the
existing closed invalid class; non-login base returns unsupported. Stale pins
return conflict. Audit, repository, terminal, time, and entropy failures retain
their existing closed classes.

## 7. Acceptance gates

The slice is complete only when tests prove:

1. exact default and named-vault grammar with canonical item/base selectors;
2. precondition failures advance `ItemConflictMerge` without prompting;
3. prompt and entropy failures advance the same item-scoped audit action before
   returning their host error;
4. invalid authored form advances a failed event before its closed error;
5. success creates one login revision whose direct parents equal the entire
   former current set, preserves base non-form metadata, and retains history;
6. restart observes one redacted authored login and no current conflict;
7. audit rows and all process output exclude candidate and authored secrets;
8. a named target changes only its own audit chain and repository; and
9. formatting, Clippy, rustdoc, application/CLI tests, and a real executable
   PTY journey pass on the affected dependency closure.
