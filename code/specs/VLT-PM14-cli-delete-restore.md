# VLT-PM14 - Reversible Delete and Restore CLI

Status: normative Phase 1A slice

Depends on: VLT-PM03, VLT-PM05, VLT-PM08, VLT-PM09, VLT-PM10, VLT-PM11,
VLT-PM12, VLT-PM13

## 1. Purpose

This slice completes the first reversible current-item lifecycle through the
standalone executable:

```text
vault-pm item delete ITEM
vault-pm history restore ITEM REVISION
```

Deletion and restoration ship together. Delete publishes an authenticated
causal tombstone instead of erasing immutable history. Restore selects an exact
live revision already visible through `history list ITEM` and publishes its
document as a new current revision. It never rewinds a repository head or
silently discards intervening history.

The two commands are prioritized before search because they close safe CRUD and
recovery for the login vertical already exposed by VLT-PM11 through VLT-PM13.
Search remains the next navigation slice; conflict resolution remains separate
because it needs a command that can deliberately choose among multiple current
candidates.

## 2. Locked properties

1. `ITEM` and `REVISION` are strict canonical VLT-PM03 identities.
2. Both commands unlock once through the controlling terminal and hold the
   cross-process writer guard for the complete operation.
3. Delete resolves the sole authenticated current live revision immediately
   before mutation and supplies it as the application compare-and-swap target.
4. A missing item, current tombstone, or conflicted current item cannot be
   deleted through this command.
5. Delete publishes a new tombstone with the deleted live revision as its
   direct causal parent; it does not erase or overwrite any existing object.
6. Restore first proves that `REVISION` appears in the bounded authenticated
   history of the exact `ITEM` supplied by the user.
7. A revision from another item is not a valid selector even if it is reachable
   elsewhere in the same vault.
8. A tombstone cannot be restored. Users select a live ancestor instead.
9. The application independently revalidates the selected revision against the
   authenticated reachable repository before publication.
10. Restore requires the selected item to have exactly one current candidate.
    It fails closed instead of choosing through a current conflict.
11. Restore copies the selected live document into a new revision whose only
    direct causal parent is the selected revision.
12. Restore does not make an old object current, rewind repository heads, erase
    the tombstone, or discard revisions created after the selected revision.
13. Each mutation obtains a fresh exact-size randomness block from the host OS
    entropy boundary.
14. The unlocked session and owned mutation inputs are consumed by the
    application mutation on every return path.
15. Success is rendered only after the new active owner state is durably
    published.
16. Neither command accepts or renders an item password, historical secret,
    notes body, provider credential, object identity, key, nonce, or signature.

## 3. Grammar

The parser adds only:

```text
item delete CANONICAL_ITEM_ID
history restore CANONICAL_ITEM_ID CANONICAL_REVISION_ID
```

It rejects missing or extra positionals, noncanonical identities, flags,
abbreviated selectors, titles, search queries, `--force`, `--purge`, `--json`,
`--copy`, `--reveal`, fields, files, environment references, and non-Unicode
arguments.

No passphrase, item secret, confirmation token, provider credential, or storage
location is accepted through argv, stdin, environment, configuration, or a URL.
The passphrase continues to come only from the fixed controlling-terminal
unlock prompt.

Delete has no yes/no confirmation prompt in this slice. The command already
requires an authenticated one-shot session and the exact canonical item ID, and
the logical operation is recoverable through retained history. Future friendly
aliases or bulk operations must add their own explicit confirmation policy;
this contract does not authorize them.

## 4. Delete application boundary

After authenticated open, the CLI invokes:

```text
UnlockedVaultV1::current_item_revision(ITEM)
```

The call returns only the exact sole current live revision. A missing item or a
current tombstone returns no candidate. Multiple retained current candidates
return conflict-required rather than selecting a display winner.

The CLI then consumes the unlocked session through:

```text
UnlockedVaultV1::delete_item(
    EXPECTED_REVISION,
    DELETED_AT_MS,
    WALL_TIME_MS,
    FRESH_RANDOMNESS,
    LOCAL_STATE_STORE,
)
```

The application compares `EXPECTED_REVISION` with the authenticated current
catalog again, constructs a tombstone naming that revision as its causal
parent, publishes the immutable revision/catalog/commit frames, and advances
owner state through its crash-resumable mutation journal. Advisory deletion
and commit times are supplied separately by the application API; this CLI
reads one host time and supplies that value for both. Time never establishes
causality or resolves a conflict.

## 5. Restore application boundary

After authenticated open, the CLI invokes:

```text
UnlockedVaultV1::item_history(ITEM, DEFAULT_ITEM_HISTORY_LIMIT)
```

The CLI finds an exact `REVISION` match in that secret-free projection. No
match returns not found, including a real revision that belongs to another
item or lies outside the user-visible bound. A matched tombstone returns the
invalid-command class because tombstones have no live document to restore.

The CLI discards the redacted selection view and consumes the unlocked session
through:

```text
UnlockedVaultV1::restore_item(
    REVISION,
    WALL_TIME_MS,
    FRESH_RANDOMNESS,
    LOCAL_STATE_STORE,
)
```

The application locates and authenticates the selected live revision again
within its hard reachable-history bound, verifies vault and item binding,
requires exactly one current candidate for that item, copies the selected
document, creates a fresh revision identity, and publishes a new commit. The
selected old revision is a causal source, not a mutable destination.

The CLI precheck is a user-selector binding and policy check. It is not a
replacement for the application's authenticated mutation validation.

## 6. Execution order

After strict parsing, either command:

1. resolves and permission-checks the local roots;
2. acquires the persistent writer guard;
3. loads and strictly parses storage-neutral configuration;
4. verifies the selected Phase 1A filesystem store;
5. opens owner state, bootstrap, and repository adapters;
6. collects the vault passphrase from the controlling terminal;
7. completes authenticated repository open;
8. resolves the exact current revision or exact item-bound historical selector;
9. rejects missing, deleted, stale, or conflicted state as applicable;
10. reads the host clock and fills the exact mutation randomness block;
11. consumes the unlocked session through the application mutation;
12. waits for durable active-owner-state publication; and
13. renders the complete fixed success line.

Failures render no partial success. A host clock or entropy failure occurs
before mutation publication. A mutation publication failure is recovered or
classified by the existing application journal and never reported as success.

## 7. Exact output

Successful deletion emits one LF-terminated line:

```text
Item deleted: ITEM_ID
```

Successful restoration emits one LF-terminated line:

```text
Item restored: ITEM_ID
```

`ITEM_ID` is the same canonical identity supplied by the user. No revision,
title, schema, provider, path, or cryptographic detail is appended. Users can
run `history list ITEM` to observe the resulting causal revision.

## 8. Failure contract

| Condition | Exit | Public result |
|---|---:|---|
| success | 0 | one fixed success line |
| invalid grammar, noncanonical ID, or tombstone restore selector | 2 | fixed invalid-command error |
| wrong passphrase | 3 | fixed authentication-required error |
| item or item-bound live revision absent | 4 | fixed not-found error |
| stale target, current conflict, concurrent writer, or recovery conflict | 5 | fixed recovery-or-conflict error |
| malformed, unauthenticated, replayed, or cross-vault state | 6 | fixed integrity error |
| terminal, entropy, clock, local state, or repository unavailable | 7 | fixed storage-unavailable error |
| configuration, backend, or format unsupported | 8 | fixed unsupported error |
| internal invariant failure | 10 | fixed internal error |

Failures have empty stdout. Delete intentionally maps a current tombstone to
not found because the ordinary current-item capability is absent. Restore
intentionally distinguishes an item-bound tombstone selector from an absent
selector so the user can choose one of the visible live ancestors.

## 9. Acceptance tests

CLI package tests prove:

1. only canonical item and revision identities are accepted;
2. add, edit, history, delete, and restore succeed across fresh one-shot hosts;
3. show returns not found while the tombstone is current;
4. repeated deletion of the current tombstone returns exit 4 without mutation;
5. post-delete history contains a newest tombstone with one direct parent;
6. selecting that tombstone for restore returns exit 2 without mutation;
7. selecting the original live revision restores its original redacted title;
8. the restored login password remains redacted;
9. audit observes four immutable revisions for the one-item lifecycle;
10. missing items and revisions return exit 4;
11. wrong passphrases return exit 3 before selector or mutation work; and
12. all master and item passwords remain absent from public output.

The real executable PTY suite additionally:

1. creates, edits, lists history, deletes, and restores in separate processes;
2. takes the restore selector only from the canonical redacted history output;
3. observes the deleted item as not found from another process;
4. observes the tombstone through redacted history before restoration;
5. injects decoy process-stdin data into every authenticated process; and
6. proves the master, original item, and replacement item passwords remain
   absent from both transcript and isolated filesystem tree.

Linux, macOS, and Windows CI must compile and test the affected packages. Unix
runs the real PTY suite; Windows compiles the native console path.

## 10. Explicit non-goals and backlog

This slice does not add permanent purge, garbage collection, batch deletion,
friendly-name deletion, undo aliases, restore-by-title, historical secret
reveal, historical show, conflict resolution, search, pagination, JSON,
non-login current-item renderers, portable host composition, or the foreground
shell.

After this slice:

- 9b-2b-2 remains redacted search and non-login current-item renderers;
- 9b-3a-2 remains additional record creation and richer field editing;
- 9b-3b-1 is complete reversible delete/restore using exact visible revision
  selectors;
- 9b-3b-2 remains explicit current-conflict resolution;
- 9b-4 remains portable import/export CLI host composition; and
- 9b-5 remains the foreground shell.
