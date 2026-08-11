# VLT-PM16 - CLI Secure-Note Creation and Redacted Read

Status: normative Phase 1A slice

Depends on: VLT-PM03, VLT-PM05, VLT-PM08, VLT-PM11, VLT-PM15

## 1. Purpose

This slice adds the first non-login record ceremony to the local CLI without
weakening the storage-neutral application boundary or the active audit epoch.
It creates one VLT02 `SecureNote`, lists its safe title, and shows only a body
omission marker. The body never becomes an argv value, echoed terminal field,
renderer input, audit fact, path, object name, or diagnostic.

## 2. Closed grammar

The only new command is:

```text
vault-pm item add secure-note
```

The parser rejects missing, duplicate, or trailing positionals; every inline
title/body, JSON, environment, file, descriptor, reveal, copy, or output flag;
unknown record-kind spellings; and non-Unicode process arguments. In
particular, `--body` is never accepted.

## 3. Host input ceremony

The command uses the shared create preflight from VLT-PM11. After the writer
lock is held but before authentication, the host reserves advisory time,
`AddItemRandomnessV1`, the favorite-register operation ID, and
`AuditedAccessRandomnessV1` for a possible form failure. It then unlocks one
short-lived session and collects exactly:

| Field | Prompt | Terminal mode | UTF-8 bytes | Empty |
|---|---|---:|---:|---:|
| title | `Title: ` | unchanged | 1-256 | no |
| body | `Note: ` | echo disabled temporarily | 1-1024 | no |

Both prompts are fixed enum values. Input comes only from the controlling
terminal or attached console, never redirected stdin. The hidden body is
validated as UTF-8 while owned by a wipe-on-drop byte wrapper, then moved into
a wipe-on-drop string. Echo state is restored on success, error, and drop.
This first slice is deliberately one line; multiline editing belongs to the
future interactive shell and desktop surfaces.

## 4. Document and publication

The CLI constructs one `ItemDocument` with:

- schema `vault/note/v1`;
- the item ID derived from `AddItemRandomnessV1`;
- equal creation, update, favorite, and publication advisory times;
- favorite `false` and empty collections, tags, and attachments; and
- `AnyRecord::SecureNote { title, body }`.

The shared create context consumes the unlocked session through `add_item`.
With an active audit epoch, success publishes the encrypted revision, catalog,
signed successful item-scoped `ItemCreate`, and commit atomically. A title or
body prompt/validation failure consumes the same session through the
audit-only boundary and publishes a failed `ItemCreate` against the reserved
item ID before returning the closed host error. If audit publication fails,
the original form error is withheld and exact pending recovery state remains.

Success output is exactly:

```text
Item added: ITEM_ID
```

## 5. Redacted list and show

The existing authenticated `item list` row is:

```text
ITEM_ID<TAB>vault/note/v1<TAB>QUOTED_TITLE
```

`item show ITEM` renders:

```text
Item: ITEM_ID
Type: vault/note/v1
Title: QUOTED_TITLE
Body: <redacted>
Favorite: no|yes
Updated: UNIX_MILLISECONDS
```

The body does not cross the domain redaction boundary. In an active epoch,
list and show retain VLT-PM15 publish-before-render ordering; successful show
binds its exact selected revision. Missing, tombstoned, conflicted, integrity,
or provider failures render no partial note metadata.

## 6. Security and acceptance

Acceptance requires:

1. closed parser coverage proving body bytes cannot enter argv;
2. fixed-prompt host tests for the title and hidden body;
3. active-epoch package tests proving successful create/list/show events and
   audit rows contain no title or body;
4. exact redacted list/show grammar and restart persistence;
5. a real-process PTY test proving the body is not echoed; and
6. recursive storage-tree scanning proving passphrase, login secrets, and note
   body plaintext are absent.
