# VLT-PM13 - Redacted Revision History CLI

Status: normative Phase 1A slice

Depends on: VLT-PM03, VLT-PM05, VLT-PM08, VLT-PM09, VLT-PM10, VLT-PM11,
VLT-PM12

## 1. Purpose

This slice makes the causal history created by item replacement visible through
the standalone executable:

```text
vault-pm history list ITEM
```

It is the first user-facing revision selector. The output is deliberately
redacted but includes canonical revision identities so a later restore command
can select an exact authenticated live ancestor without guessing or reopening a
secret in the list path.

History is prioritized before delete/restore because deleting without a visible
selection and recovery path would be unsafe product sequencing. Search remains
separate because it improves navigation but does not unlock lifecycle recovery.

## 2. Locked properties

1. `ITEM` is a strict canonical VLT-PM03 item ID.
2. The command unlocks once through the controlling terminal and holds the
   cross-process writer guard for the complete operation.
3. It traverses only commits reachable from the authenticated repository heads
   returned by the opened session.
4. It uses `DEFAULT_ITEM_HISTORY_LIMIT`; callers cannot weaken or enlarge the
   bound with a flag in this slice.
5. Revisions reached through multiple heads or catalogs are emitted once.
6. Repository, catalog, revision, vault, item, parent, and authenticated-frame
   mismatches fail closed before output.
7. History is materialized only as `ItemHistoryViewV1`; secret-bearing
   historical documents never cross into the CLI renderer.
8. The unlocked session is synchronously locked before the first output byte is
   constructed.
9. Every revision identity is rendered in its strict canonical user form.
10. Output contains only revision state, direct-parent count, advisory time,
    and approved redacted record metadata.
11. An item absent from the bounded reachable history returns not found rather
    than an empty success that could be confused with an existing item.
12. The command performs no repository or local-state mutation.

## 3. Grammar

The parser adds only:

```text
history list CANONICAL_ITEM_ID
```

It rejects missing or extra positionals, noncanonical item IDs, revision or
limit flags, `--json`, `--copy`, `--reveal`, fields, files, environment
references, and non-Unicode arguments.

No passphrase, item secret, query, revision identity, or provider credential is
accepted through argv, stdin, environment, configuration, or a URL. The
passphrase continues to come only from the fixed controlling-terminal unlock
prompt.

## 4. Application boundary

The CLI invokes:

```text
UnlockedVaultV1::item_history(ITEM, DEFAULT_ITEM_HISTORY_LIMIT)
```

The application traverses repository heads newest ancestry depth first.
Commits at the same depth and revisions in the same catalog are ordered by
exact authenticated object identity. It authenticates every commit, catalog,
revision, and direct parent it touches, enforces vault and item binding, and
deduplicates repeated commits, catalogs, and revisions.

Each returned `ItemHistoryViewV1` contains only:

- the exact revision ID;
- an optional `RedactedItemView` for live revisions;
- whether the revision is a tombstone;
- the direct causal-parent count; and
- document-update or tombstone-deletion advisory time.

The projection contains no password, notes body, TOTP seed, API key, database
credential, opaque payload, attachment bytes, object frame, locator, provider
fact, key, nonce, or signature. Although the reusable redacted item projection
can carry approved username and URL metadata, this command renders neither.

## 5. Execution order

After strict parsing, the command:

1. resolves and permission-checks the local roots;
2. acquires the persistent writer guard;
3. loads and strictly parses storage-neutral configuration;
4. verifies the selected Phase 1A filesystem store;
5. opens owner state, bootstrap, and repository adapters;
6. collects the vault passphrase from the controlling terminal;
7. completes authenticated repository open;
8. requests bounded history for `ITEM`;
9. synchronously locks and wipes the live session;
10. maps an empty result to the fixed not-found class; and
11. renders the complete redacted result.

No partial history is rendered. Any failure during traversal or projection
returns empty stdout.

## 6. Ordering and bounds

The application-defined order is preserved exactly. For the ordinary linear
edit case this is newest revision first, followed by its causal ancestors.
Conflict histories may interleave candidates deterministically according to
reachable-head depth and exact object identity; the CLI must not invent a
winner, collapse a conflict, or sort by advisory wall time.

`DEFAULT_ITEM_HISTORY_LIMIT` bounds repository ancestry traversal. The hard
application limit remains authoritative. This command has no pagination or
caller-controlled bound; a later interface may add opaque pagination without
changing the V1 line format.

## 7. Exact line format

Each live revision is one LF-terminated line:

```text
REVISION_ID<TAB>live<TAB>parents=N<TAB>updated=MILLISECONDS<TAB>SCHEMA<TAB>"ESCAPED TITLE"
```

Each tombstone is one LF-terminated line:

```text
REVISION_ID<TAB>deleted<TAB>parents=N<TAB>deleted=MILLISECONDS
```

`REVISION_ID` is the canonical uppercase VLT-PM03 identity. `N` and
`MILLISECONDS` are minimal unsigned decimal integers. Schema is the validated
content type. Title is the same escaped quoted redacted display title used by
item listing and supports every VLT-PM03 record projection. Tombstones do not
retain or synthesize record metadata.

The format intentionally exposes revision identities because they are explicit
future restore selectors. It does not expose commit, catalog, announcement, or
storage object identities.

## 8. Failure contract

| Condition | Exit | Public result |
|---|---:|---|
| success | 0 | complete ordered history lines |
| invalid grammar or noncanonical item ID | 2 | fixed invalid-command error |
| wrong passphrase | 3 | fixed authentication-required error |
| item absent from bounded reachable history | 4 | fixed not-found error |
| concurrent writer or recovery conflict | 5 | fixed recovery-or-conflict error |
| malformed, unauthenticated, replayed, or cross-vault state | 6 | fixed integrity error |
| terminal, local state, or repository unavailable | 7 | fixed storage-unavailable error |
| configuration, backend, or format unsupported | 8 | fixed unsupported error |
| internal invariant failure | 10 | fixed internal error |

Failures have empty stdout. A current item conflict does not itself prevent
history listing; every reachable revision remains visible in deterministic
redacted form so later conflict and restore workflows have exact selectors.

## 9. Acceptance tests

CLI package tests prove:

1. only a canonical item ID is accepted by `history list`;
2. add, edit, and history succeed across fresh one-shot hosts;
3. the edited live revision appears before its original ancestor;
4. both revision IDs parse as canonical VLT-PM03 identities;
5. direct-parent counts are one and zero for the linear two-revision case;
6. schema and escaped titles are present;
7. old and replacement passwords are absent;
8. a missing item returns exit 4; and
9. a wrong passphrase returns exit 3 before history traversal.

The real executable PTY suite additionally:

1. creates and edits a login in separate processes;
2. lists its history in another process using only the controlling terminal;
3. observes both redacted revisions in newest-first order;
4. injects decoy process-stdin data into the authenticated process; and
5. proves the master, old item, and replacement item passwords remain absent
   from both transcript and isolated filesystem tree.

Linux, macOS, and Windows CI must compile and test the affected packages. Unix
runs the real PTY suite; Windows compiles the native console path.

## 10. Explicit non-goals and backlog

This slice does not add historical secret reveal, historical show, restore,
delete, conflict resolution, search, pagination, JSON, non-login current-item
renderers, portable host composition, or the foreground shell.

After this slice:

- 9b-2b-2 remains redacted search and non-login current-item renderers;
- 9b-3a-2 remains additional record creation and richer field editing;
- 9b-3b remains delete, restore, and conflict resolution using the revision
  selectors introduced here;
- 9b-4 remains portable import/export host composition; and
- 9b-5 remains the foreground shell.
