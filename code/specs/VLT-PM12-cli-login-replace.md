# VLT-PM12 - Revision-Safe Login Replacement CLI

Status: normative Phase 1A slice, extended by VLT-PM30

Depends on: VLT-PM03, VLT-PM05, VLT-PM08, VLT-PM09, VLT-PM10, VLT-PM11

## 1. Purpose

This slice makes a stored login maintainable and creates the first meaningful
revision history through the standalone executable:

```text
vault-pm item edit ITEM
```

It is prioritized before search/history rendering because VLT-PM11 vaults
otherwise contain only immutable one-revision items. Replacement establishes
the current-revision capability and causal history that later history,
restore, delete, and conflict commands consume.

The command is a complete replacement form for the fields VLT-PM30 can create.
It does not implement an interactive full-screen editor, secret-preserving
blank defaults, or patch semantics.

## 2. Locked properties

1. `ITEM` is a strict canonical VLT-PM03 item ID.
2. The command unlocks once through the controlling terminal and holds the
   cross-process writer guard for the complete operation.
3. VLT-PM05 resolves exactly one current live revision. Missing and tombstoned
   items return not found; conflicts fail before item prompts.
4. The application returns the current revision ID only as an optimistic
   mutation capability. It is never rendered.
5. The exact current live document is opened inside a wipe-on-drop wrapper.
6. Non-login records fail as unsupported before item prompts. Every valid
   login is editable regardless of its current URL count.
7. Title, username, password, zero-to-sixteen URLs, and optional private notes
   are collected again through the VLT-PM30 controlling-terminal contract. No
   old secret is rendered or used as a prompt default.
8. Stable item identity, schema, creation time, favorite register,
   collections, tags, and attachments are preserved exactly; the complete URL
   list and notes are explicitly replaced.
9. Fresh host CSPRNG bytes protect every replacement publication frame.
10. `replace_item` compares the selected revision with the sole current live
    revision before publishing. A stale selection or new conflict fails
    without replacing either candidate.
11. The unlocked session is consumed by the mutation; success cannot retain
    stale pins, decrypted state, or a reusable unlock.
12. Output is produced only after the crash-resumable publication reaches a
    durable active owner state.

## 3. Grammar

The parser adds only:

```text
item edit CANONICAL_ITEM_ID
```

It rejects missing or extra positionals, noncanonical IDs, record-kind
positionals, field flags, inline values, `--password`, `--copy`, `--reveal`,
`--json`, files, environment references, and non-Unicode arguments.

This command does not accept an expected revision from the user. The exact
revision is resolved from the authenticated current view in the same guarded
operation that later attempts replacement.

## 4. Current-revision application capability

`UnlockedVaultV1::current_item_revision(ITEM)` returns:

- `None` for an absent item or sole current tombstone;
- the exact revision ID for one current live candidate; or
- `ConflictRequired` for multiple retained current candidates.

The method reads only the already authenticated materialized catalog. It does
not rediscover provider state, select a conflict winner, decrypt a historical
object, mutate pins, or expose the revision through `Debug` or an ordinary
item view.

The CLI passes the returned identity to `reveal_item_revision` to obtain an
owned zeroizing document. This keeps secret-bearing document access explicit
and separate from redacted `get_item`/`list_items`.

## 5. Execution order

After strict parsing, the command:

1. resolves and permission-checks local roots;
2. acquires the persistent writer lock;
3. loads and strictly parses storage-neutral configuration;
4. verifies the selected Phase 1A local filesystem store;
5. opens owner state, bootstrap, and repository adapters;
6. collects the vault passphrase from the controlling terminal;
7. completes authenticated repository open;
8. resolves the sole current live revision for `ITEM`;
9. reveals that exact revision in a wipe-on-drop document;
10. proves the schema is `vault/login/v1`;
11. collects the complete replacement login form using the fixed VLT-PM30
    prompts;
12. reads advisory wall time and fresh replacement randomness;
13. constructs the complete replacement document;
14. drops the old revealed document before publication; and
15. consumes the unlocked session through `replace_item`.

Failures before step 11 never emit item prompts. Failures after secret
collection drop and wipe every owned form value and constructed document.

## 6. Replacement document

The replacement preserves from the authenticated current document:

- item ID;
- content type;
- creation timestamp;
- complete favorite last-writer-wins register, including operation ID;
- collection observed set;
- tag observed set;
- attachment observed set.

It replaces title, username, password, the complete ordered URL list, and
optional private notes.

The new document update time is `max(host_wall_time, current_update_time)`.
This prevents an advisory host-clock rollback from making the document older
than itself or its preserved favorite register. Publication commit time remains
the exact host wall time and does not establish causality.

The command accepts current logins with any valid URL count. It never truncates
or silently preserves an uneditable tail: VLT-PM30 owns the complete ordered
list and notes explicitly.

## 7. Optimistic and crash-safe publication

The host fills exactly `REPLACE_ITEM_RANDOM_BYTES` from the OS CSPRNG and wraps
them in `ReplaceItemRandomnessV1`. VLT-PM05 then:

- proves durable session pins still equal the opened report;
- proves the item still has one current live candidate;
- proves that candidate equals the selected revision;
- proves item identity, schema, and creation time were preserved;
- writes the encrypted replacement revision and dependent catalog/commit
  objects before publication;
- records an exact pending owner journal before the external announcement;
- publishes the announcement last; and
- compare-exchanges the intended active owner state.

The replacement revision names the selected revision as its sole causal
parent. Ambiguous provider or local-state failure retains the exact journal for
the existing recovery workflow. The CLI neither retries with new randomness
nor constructs a second logical edit.

## 8. Output and failure contract

Success is exactly:

```text
Item updated: ITEM_ID
```

| Condition | Exit | Public result |
|---|---:|---|
| success | 0 | exact updated line |
| invalid grammar or form value | 2 | fixed invalid-command error |
| wrong passphrase | 3 | fixed authentication-required error |
| absent or tombstoned item | 4 | fixed not-found error |
| current conflict, stale revision, or concurrent writer | 5 | fixed recovery-or-conflict error |
| malformed, unauthenticated, replayed, or cross-vault state | 6 | fixed integrity error |
| terminal, local state, or repository unavailable | 7 | fixed storage-unavailable error |
| non-login, config, or format unsupported | 8 | fixed unsupported error |
| internal invariant failure | 10 | fixed internal error |

Failures have empty stdout. No output includes the expected revision, old or
new password, notes, object IDs, paths, locators, provider facts, or crypto
parameters.

## 9. Acceptance tests

Application tests prove:

1. current live items return their exact sole revision capability;
2. absent and tombstoned items return `None`;
3. conflicts return `ConflictRequired`; and
4. the capability is consistent with the current candidate used by existing
   replacement compare-and-swap tests.

CLI package tests prove:

1. only a canonical item ID is accepted by `item edit`;
2. add, edit, show, and audit succeed across fresh one-shot hosts;
3. title, username, password, complete ordered URL list, and notes are replaced;
4. a zero URL count becomes `URL: none` and empty notes become `Notes: absent`;
5. normal output contains neither old nor replacement password or notes;
6. the complete audit observes two reachable revisions for one current item;
7. a missing item returns exit 4 before item input; and
8. a wrong passphrase returns exit 3 before item input.

The real executable PTY suite additionally:

1. creates a login in one process;
2. edits it in another process using only the controlling terminal;
3. reopens and shows it in a third process;
4. proves the replacement metadata is durable and the password remains
   redacted;
5. injects decoy process-stdin data into every authenticated process; and
6. recursively proves the master, original and replacement passwords, and
   original and replacement notes are absent from the isolated filesystem tree.

Linux, macOS, and Windows CI must compile and test the affected packages.
Unix runs the real PTY suite; Windows compiles the native console path.

## 10. Explicit non-goals and backlog

VLT-PM30 completes notes and multiple-URL editing. This boundary still does not
add non-login editing, partial field preservation, external editors, browser
matching, or the foreground shell.

After this slice:

- 9b-2b remains redacted search/history and non-login renderers;
- 9b-3a-2 remains additional record creation plus richer field editing;
- 9b-3b remains delete, restore, and conflict resolution;
- 9b-4 remains portable import/export host composition; and
- 9b-5 remains the foreground shell.
