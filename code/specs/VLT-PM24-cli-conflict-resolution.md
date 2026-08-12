# VLT-PM24 — Audited CLI Conflict Resolution

## Status

Normative Phase 1A contract for explicit redacted current-conflict inspection
and choose-existing-candidate resolution through the local CLI.

## 1. Purpose and boundary

Synchronization and portable import preserve concurrent current candidates
instead of silently discarding data. A preserved conflict must not strand a
local user behind generic `conflict required` errors. This contract composes
the existing storage-neutral application primitives into two explicit commands:

```text
vault-pm [--vault NAME] conflict list ITEM
vault-pm [--vault NAME] conflict choose ITEM REVISION
```

V1 deliberately supports only choosing one authenticated current candidate.
It does not reveal secret fields, accept a caller-authored merged document,
delete losing immutable history, auto-select by time/device, or resolve more
than one item per command. Field-by-field authored merge remains a later slice.

## 2. Closed grammar

`ITEM` and `REVISION` use the existing uppercase canonical selectors. Missing,
lowercase, malformed, extra, secret-bearing, or unknown arguments fail before
host preparation. No title, field value, path, provider, device, passphrase,
or selection policy may be supplied as an argument.

The leading named-vault selector remains command-scoped and never rewrites the
configured default.

## 3. Audit-first requirement

Both commands require an active operation-audit epoch. New vaults satisfy this
at generation zero; legacy pre-audit vaults must run `audit enable` first.

Before target authentication, each command reserves its complete advisory time
and audit randomness. `conflict choose` additionally reserves the complete
resolution mutation randomness. Therefore every post-unlock outcome either
advances the signed audit chain or fails closed at audit publication.

`conflict list` publishes item-scoped `ItemHistoryRead`, matching the existing
redacted history projection. `conflict choose` publishes item-scoped
`ItemConflictResolve`; success also binds the exact selected revision. Missing
items, unconflicted items, and missing or cross-item candidate selectors publish
a failed resolution event before their closed error becomes observable.

Audit rows contain no candidate title, schema, state, parent set, path, vault or
device identity, provider detail, passphrase, or arbitrary error text.

## 4. Candidate inspection

`conflict list ITEM` succeeds only when the item has at least two retained
current candidates. It returns the application-owned redacted candidate views
in exact revision-ID order. Each row uses the existing history shape:

```text
REVISION\tlive\tparents=N\tupdated=TIME\tSCHEMA\t"TITLE"
REVISION\tdeleted\tparents=N\tdeleted=TIME
```

Login passwords, secure-note bodies, TOTP seeds, API keys, database secrets,
opaque record bodies, usernames, URLs, notes, CRDT members, attachment content,
provider metadata, object IDs, and cryptographic material never reach the CLI
renderer. Missing or unconflicted items produce no partial rows.

## 5. Choose-existing resolution

`conflict choose ITEM REVISION` validates inside the application that:

- `ITEM` currently has at least two authenticated candidates;
- `REVISION` is one of those exact current candidates; and
- repository heads still equal the authenticated local pins.

Success copies the selected candidate's complete live document or tombstone
into a fresh target revision, names every retained current candidate as a direct
causal parent, and publishes the signed commit plus succeeded audit event
atomically through the ordinary crash-resumable journal. Losing candidates and
their history remain immutable and retained.

The command emits only:

```text
Conflict resolved: ITEM
```

It never emits the selected revision, winning title/state, losing candidates,
field values, parent identities, provider details, or cryptographic data.

## 6. Errors and interruption

- wrong vault passphrase: locked;
- malformed grammar or pre-audit vault: invalid;
- missing item or candidate selector: not found after a failed event;
- item without a current conflict: conflict after a failed event;
- stale pins or concurrent owner state: conflict;
- authenticated corruption: integrity;
- storage, time, or entropy unavailability: provider;
- unsupported selected provider: unsupported.

An interruption before publication leaves the old conflict visible. Ambiguous
publication retains the exact pending journal; the command releases no success
until recovery reaches the intended active state.

## 7. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `conflict list ITEM` and
   `conflict choose ITEM REVISION` with optional leading named selection;
2. pre-audit targets reject before candidate traversal;
3. current candidate rows are deterministic and contain only redacted history
   fields;
4. missing, unconflicted, and wrong/cross-item selectors publish failed
   item-scoped events before their errors;
5. choose success publishes one fresh revision with every current candidate as
   a parent and a succeeded event that binds the selected revision;
6. list and choose audit rows contain no candidate metadata or secret fields;
7. restart observes one resolved current candidate while immutable conflict
   history remains reachable;
8. a selected named vault advances only its own audit chain; and
9. formatting, Clippy, rustdoc, focused application/CLI tests, and downstream
   executable tests pass.
