# VLT-PM11 - Login Create and Redacted Read CLI

Status: normative Phase 1A slice

Depends on: VLT-PM03, VLT-PM05, VLT-PM06, VLT-PM07, VLT-PM08, VLT-PM09,
VLT-PM10

## 1. Purpose

This slice turns an initialized local vault into a minimally useful password
manager. It composes one authenticated login mutation with durable redacted
reads through the standalone executable:

```text
vault-pm item add login
vault-pm item list
vault-pm item show ITEM
```

The acceptance path is deliberately vertical: initialize, create a login,
exit, reopen in a new process, list the item, and show its redacted fields.
Read-only commands without a creation path would exercise only an empty vault;
all record kinds and mutation verbs at once would make the first user flow too
large to review as one security boundary.

## 2. Package boundaries

`vault-pm-cli-host` adds only fixed controlling-terminal input primitives:

- echoed, bounded UTF-8 text for title, username, and one optional URL; and
- a fixed hidden password prompt using the existing echo guard.

It owns terminal acquisition, prompt literals, byte bounds, UTF-8 validation,
control-character rejection for echoed text, line draining, and platform error
classification. It does not construct records or access a vault.

`vault-pm-cli` owns the closed grammar, orchestration, record construction,
fresh host-fact acquisition, application calls, and redacted rendering. It
does not encode repository objects, derive keys, implement publication, or
read provider files directly.

`vault-pm-application` remains the only mutation/read authority. The CLI calls
`add_item`, `list_items`, and `get_item` against the configured repository and
owner-state adapters.

## 3. Closed grammar

The parser accepts only:

```text
item add login
item list
item show CANONICAL_ITEM_ID
```

`CANONICAL_ITEM_ID` is the strict uppercase VLT-PM03 base32 representation.
Lowercase, aliases, wrong widths, forbidden alphabet characters, and residual
nonzero bits fail before path or terminal access.

The following are rejected:

- every item kind other than `login`;
- missing, duplicate, or trailing positionals;
- inline title, username, URL, password, JSON, environment, file, or file
  descriptor inputs;
- `--password`, `--reveal`, `--copy`, `--json`, and unknown flags; and
- non-Unicode process arguments.

There is no standalone `unlock` command. Each accepted operation owns one
short authenticated session.

## 4. Terminal input contract

After a successful vault unlock, `item add login` collects fields in this
exact order from the controlling terminal:

| Field | Prompt | Terminal mode | UTF-8 bytes | Empty | Controls |
|---|---|---:|---:|---:|---:|
| title | `Title: ` | unchanged | 1-256 | no | rejected |
| username | `Username: ` | unchanged | 0-1024 | yes | rejected |
| password | `Password: ` | echo disabled temporarily | 1-1024 | no | line terminator only |
| primary URL | `URL (optional): ` | unchanged | 0-2048 | yes | rejected |

An empty URL produces an empty URL list. This slice collects no notes,
collections, tags, attachments, favorite flag, or additional URLs.

All prompts are fixed enum values. Caller text is never interpolated into a
prompt or error. The reader opens `/dev/tty` on Unix or `CONIN$`/`CONOUT$` on
Windows; redirected process stdin is never consulted. Oversized input is
drained through its line ending before returning an error, so unread tail
bytes cannot become a later field.

Echoed fields and the password use wipe-on-drop owned values until ownership
moves into the VLT02 `Login`, whose drop implementation wipes all strings.
Invalid UTF-8 password bytes fail while still held by their zeroizing wrapper.

## 5. Shared authenticated preparation

Every command:

1. resolves and validates platform-standard owner-private roots;
2. acquires the persistent non-blocking process writer lock;
3. loads and strictly parses the exact configuration;
4. selects the configured default vault and Phase 1A local filesystem store;
5. rejects remote stores or any mismatched location/credential declaration;
6. constructs the storage-neutral application and repository adapters;
7. obtains the passphrase only from the fixed controlling-terminal unlock
   prompt; and
8. completes the VLT-PM05 authenticated open before reading or collecting item
   fields.

Unconfigured, unsupported, malformed, busy, unavailable, or authentication
failures occur before any item prompt. A successful read stores its result,
synchronously locks the lifecycle boundary, and only then renders output.

## 6. Login creation

After collecting the four item fields, the CLI obtains:

- one advisory Unix-millisecond wall time;
- exactly `ADD_ITEM_RANDOM_BYTES` of fresh OS CSPRNG output for the item ID and
  three encrypted publication frames; and
- an independent 32-byte OS CSPRNG block for the initial favorite-register
  operation ID.

It constructs one VLT-PM03 `ItemDocument` with:

- the item ID exposed by `AddItemRandomnessV1`;
- schema `vault/login/v1`;
- equal creation, update, favorite, and publication wall times;
- favorite `false`;
- empty observed sets for collections, tags, and attachments;
- a VLT02 `Login` containing the collected title, username, password, optional
  single URL, and no notes.

The CLI then consumes the unlocked session through `add_item`. VLT-PM05 proves
the random item ID matches the document, rejects an existing identity, writes
encrypted revision/catalog/commit dependencies before publication, installs
the exact pending owner journal, publishes the announcement, and completes the
owner-state compare-exchange. The consumed session cannot retain stale pins or
decrypted state after the mutation.

Success is rendered only after the application reports a durable active state.
The returned active state is not decoded or rewritten by the CLI.

## 7. Redacted reads

`item list` calls `list_items`. It returns every unambiguous current live item
in exact item-ID order. An empty vault renders one explicit line. Any current
conflict aborts the complete read; no partial list is rendered.

Each nonempty list line contains:

```text
ITEM_ID<TAB>CONTENT_TYPE<TAB>QUOTED_DISPLAY_TITLE
```

The quoted field uses escaped debug-string syntax so tabs, newlines, terminal
controls, quotes, and backslashes from imported or future records cannot forge
columns or lines. No username, URL, secret marker, timestamp, provider fact,
or revision ID appears in list output.

`item show ITEM` calls `get_item`. A missing or currently tombstoned item maps
to not found. A conflicted item fails closed. This first renderer accepts a
login view and emits:

```text
Item: ITEM_ID
Type: vault/login/v1
Title: QUOTED_TITLE
Username: QUOTED_USERNAME
URL: QUOTED_URL
Password: <redacted>
Notes: absent|present
Favorite: no|yes
Updated: UNIX_MILLISECONDS
```

Each URL has its own fixed `URL:` line; an empty list renders `URL: none`.
Title, username, and URLs use the same escaped quoted syntax. Password bytes,
notes text, revision IDs, collection/tag/attachment identities, paths,
locators, and provider details are never included. A non-login view returns
the stable unsupported class until its exact renderer is specified.

## 8. Output and failure contract

Successful creation output is exactly:

```text
Item added: ITEM_ID
```

An empty list is exactly `No items.`. Successful output ends in one newline.
Terminal prompts are outside the public stdout renderer.

| Condition | Exit | Public result |
|---|---:|---|
| success | 0 | exact add/list/show rendering |
| invalid grammar, field, or unconfigured invocation | 2 | fixed invalid-command error |
| wrong passphrase | 3 | fixed authentication-required error |
| missing/tombstoned item | 4 | fixed not-found error |
| concurrent writer, pending publication, or item conflict | 5 | fixed recovery-or-conflict error |
| malformed, unauthenticated, replayed, or cross-vault state | 6 | fixed integrity error |
| terminal, local state, or repository unavailable | 7 | fixed storage-unavailable error |
| unsupported configuration, format, or record renderer | 8 | fixed unsupported error |
| internal invariant failure | 10 | fixed internal error |

Failure stderr is payload-free. Creation failure has no success item ID.
List/show failure has no partial item rows or fields.

## 9. Security invariants

1. Passwords are never accepted through argv, stdin, environment, config, or
   output flags.
2. The password prompt has echo disabled and restores prior mode on success,
   error, and drop.
3. No plaintext item field becomes an object name, storage key, path, config
   value, bootstrap value, error, or ordinary diagnostic.
4. Item and operation identities come from independent host CSPRNG blocks, not
   timestamps, titles, usernames, or deterministic counters.
5. Normal list/show output can render only `RedactedItemView`; it has no access
   to `ItemDocument` or a reveal API.
6. A read conflict returns no partial projection.
7. The local writer guard spans config selection, unlock, read/mutation, and
   session disposal.
8. Separate process invocations prove durability and prevent an accidental
   in-memory-only success path.

## 10. Acceptance tests

Package tests must prove:

1. the grammar accepts only the three exact forms and canonical item IDs;
2. secret flags and trailing arguments fail before host side effects;
3. a login add publishes one durable item and returns its random item ID;
4. separate hosts reopen, list, and show that item after the mutation session
   was consumed;
5. list/show never contain the plaintext password;
6. the show password field is exactly `<redacted>`;
7. wrong-passphrase reads have empty stdout and exit 3;
8. a missing item has empty stdout and exit 4; and
9. text validation rejects invalid UTF-8, controls, oversize, and an empty
   required title while allowing empty username and URL.

The real executable PTY suite must additionally:

1. initialize under isolated platform roots;
2. run login creation, list, and show in three later processes;
3. inject decoy secrets into redirected stdin for every authenticated command;
4. observe the fixed controlling-terminal prompts in order;
5. under the PTY's ordinary echo mode, observe the title, username, and URL
   echoed but not the password;
6. carry the random creation ID into the later show command;
7. observe the exact redaction marker; and
8. recursively prove both master and item passwords are absent from the
   isolated filesystem tree.

Native CI compiles and tests on Linux, macOS, and Windows. Unix runs the PTY
process suite; Windows compiles the production console implementation and runs
its platform-independent validation tests.

## 11. Explicit non-goals and reprioritized backlog

This slice does not implement additional record kinds, edit/replace,
delete/restore, conflicts, search, history, secret reveal/copy, JSON, notes,
multiple URLs, tags, collections, attachments, export/import commands, a
foreground shell, or cloud adapters.

After this slice, VLT-PM00 item 9b is split into:

- 9b-2b: redacted search and history reads plus non-login show renderers;
- 9b-3a: remaining record creation and login replace/edit;
- 9b-3b: delete, restore, and conflict resolution;
- 9b-4: portable export/import host commands and destination policy; and
- 9b-5: foreground interactive shell over the same command/use-case boundary.
