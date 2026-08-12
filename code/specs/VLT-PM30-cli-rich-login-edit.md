# VLT-PM30 — Audited Rich Login Authoring

## Status

Normative Phase 1A contract for complete login creation and replacement with
multiple URLs and optional private notes through the local CLI.

## 1. Purpose and boundary

The typed `LOGIN_V1` record already supports a URL list and optional notes,
while the CLI currently authors at most one URL, cannot author notes, and
rejects existing multi-URL records during edit. This slice closes that gap for:

```text
vault-pm [--vault NAME] item add login
vault-pm [--vault NAME] item edit ITEM
```

URL parsing, normalization, reachability, browser matching, form-field
discovery, partial edits, interactive menus, external editors, multiline
notes, and non-interactive input are outside this slice.

## 2. Closed form

After one-shot unlock, add and edit collect the complete replacement form in
this order:

```text
Title:
Username:
Password:
URL count (0-16):
URL:                    # repeated exactly count times
Notes (optional):
```

Title is required control-free UTF-8 metadata of at most 256 bytes. Username
is optional control-free UTF-8 metadata of at most 1,024 bytes. Password is a
required echo-disabled UTF-8 secret of at most 1,024 bytes. URL count is one
canonical decimal integer in `0..=16`, with no sign or leading zero. Each URL
is required control-free UTF-8 metadata of at most 2,048 bytes; ordering and
duplicates are preserved exactly because provider-neutral storage must not
invent URL equivalence. Notes are optional echo-disabled UTF-8 of at most
1,024 bytes and are absent only when the hidden input is empty.

Prompts are fixed and contain no item data or indexes. Fields, counts, secrets,
paths, and bypasses are never accepted through argv, stdin, environment,
configuration, or URLs.

## 3. Audit-first mutation

Create retains VLT-PM21's pre-authentication reservation and VLT-PM16's shared
item-create context. After successful authentication, every form prompt,
terminal/UTF-8/count failure, document failure, and repository failure either
publishes one item-scoped failed `ItemCreate` before its closed error or
atomically publishes the encrypted record and succeeded event.

Edit retains VLT-PM12's application-owned current revision/document and
VLT-PM15's `ItemUpdate` event. It accepts every current `LOGIN_V1`, including
multiple URLs, and replaces title, username, password, complete ordered URL
list, and notes together. Every post-authentication host, count, document, and
publication failure becomes durable before its error. Success atomically
publishes the replacement and event; immutable history retains the old form.

Audit events contain no form value, URL count, password/notes property, prompt
index, schema, path, provider detail, or arbitrary error text.

## 4. Secret ownership and observation

All collected fields remain wipe-on-drop until moved into the zeroizing typed
record. Password and notes never enter normal CLI output, logs, audit metadata,
or debug output. `item show ITEM` emits every URL in stored order and only:

```text
Password: <redacted>
Notes: present          # or `Notes: absent`
```

Password access remains `item reveal ITEM login-password`. Notes access uses
`item reveal ITEM login-notes`. Both require VLT-PM25's exact-`yes`,
publish-before-release terminal ceremony; absent notes or a mismatched schema
publish failed access before their closed error.

## 5. Output and errors

Create and edit retain their exact existing success lines. Invalid grammar,
count, UTF-8, or document values return invalid; wrong passphrase returns
locked; missing item returns not found; conflict returns conflict; non-login
returns unsupported; authenticated corruption returns integrity; host/storage/
audit publication unavailability returns provider. Failures have empty stdout.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. URL count accepts only canonical `0..=16` and drives exactly that many fixed
   URL prompts before one hidden optional-notes prompt;
2. add survives restart with zero, one, and multiple ordered URLs and explicit
   notes-presence redaction;
3. edit accepts an existing multi-URL login and replaces all authored fields,
   including clearing URLs and notes, without truncation or preservation;
4. host, count, UTF-8, document, and repository failures become durable failed
   create/update events before their errors;
5. list/show/history/audit/debug exclude password and notes bytes and audit rows
   contain only closed fields;
6. password and notes are absent from the isolated persisted profile tree;
7. separate audited reveal returns password or notes only through direct
   terminal delivery after authorization; and
8. formatting, Clippy, rustdoc, application/host/CLI tests, and real PTY
   executable tests pass on the affected dependency closure.
