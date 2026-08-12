# VLT-PM25 — Audited Interactive Secret Reveal

## Status

Normative Phase 1A contract for item-bound, schema-specific secret disclosure
through the local CLI and controlling terminal.

## 1. Purpose and boundary

A password manager that can create encrypted records but cannot return their
secrets is not yet locally usable. The application already owns typed secret
selection, disclosure policy, wipe-on-drop output, and publish-before-release
audit events. This contract composes those primitives without weakening them:

```text
vault-pm [--vault NAME] item reveal ITEM FIELD
```

V1 supports these exact UTF-8 field selectors:

```text
login-password
secure-note-body
card-number
card-cvv
api-key-token
database-password
```

Raw binary TOTP seed rendering, clipboard delivery, unsafe non-interactive
output, historical-revision reveal, and multi-field reveal remain later
ceremonies. They may reuse the application policy but must not silently extend
this command.

## 2. Closed grammar

`ITEM` is the existing uppercase canonical item selector. `FIELD` is one exact
schema-specific token above. Missing, lowercase item, unknown, generic,
secret-bearing, extra, or option-like arguments fail before host preparation.
The command accepts no secret, revision, path, output destination, provider,
warning bypass, or confirmation flag.

The optional leading named-vault selector remains command-scoped and never
rewrites the configured default.

## 3. Authentication and confirmation

Before authentication, the CLI reserves the complete advisory time and audit
randomness for the attempt. It then authenticates the selected target and
requires an active audit epoch. Only after unlock does the host write this
fixed prompt to the controlling terminal:

```text
Reveal secret on this terminal? Type yes to continue:
```

Only exact lowercase `yes` authorizes reveal. Empty input, EOF, other text,
terminal unavailability, or validation failure never authorizes release.
Explicit refusal publishes `Denied`. A host failure while collecting the
answer also publishes `Denied` before the payload-free host error is returned.

Wrong-passphrase and pre-authentication time/entropy failures release no
secret and do not claim that an item access occurred.

## 4. Application-owned selection

The CLI passes only `ITEM`, the closed `FIELD`, and confirmed interactive
intent. The application resolves the sole current live candidate and selects
the typed field in one session-consuming boundary. The current revision
capability, complete document, other fields, losing conflict candidates, and
historical plaintext never cross into CLI orchestration.

The boundary publishes one item-scoped `ItemRead` event:

- refused or unavailable confirmation: `Denied`, no selected revision;
- missing, tombstoned, or conflicted item: `Failed`, no selected revision;
- field/schema mismatch: `Failed`, exact current revision bound;
- successful selection: `Succeeded`, exact current revision bound.

The event contains no field selector, schema, title, username, URL, secret
length/value, confirmation answer, terminal identity, vault name, device name,
provider detail, path, or arbitrary error text. Audit publication failure
withholds both the secret and original operation error while retaining the
ordinary exact recovery journal.

## 5. Publish-before-release terminal delivery

The application returns the owned non-printable `RevealedSecretV1` only after
the next audit owner state is durable. The CLI accepts only the UTF-8 encoding
for this command and borrows the bytes until terminal delivery completes.

The secret never enters `CliOutput`, process stdout, process stderr, argv,
stdin, an environment variable, configuration, a file, a URL, `Debug`, or a
cloneable string owned by the command result. The native host reopens the
controlling terminal or attached console and writes exactly one line:

```text
Secret: "QUOTED AND ESCAPED VALUE"
```

String-debug quoting escapes newline, carriage return, tab, quote, backslash,
escape, and other control characters so stored values cannot inject terminal
control sequences or counterfeit later output. The temporary escaped buffer
and Windows UTF-16 buffer are wipe-on-drop. The application-owned secret is
wiped immediately after the terminal write attempt.

A terminal write failure occurs after the process was authorized and received
the secret. The succeeded access event therefore remains truthful; the command
returns the stable provider failure and does not retry or redirect the value.

## 6. Errors and output

- malformed grammar, refusal, or field/schema mismatch: invalid;
- wrong passphrase: locked;
- missing or tombstoned item: not found;
- current conflict: conflict;
- authenticated corruption: integrity;
- time, entropy, terminal, audit publication, or terminal write unavailable:
  provider;
- unsupported platform or binary field encoding: unsupported.

The command has empty ordinary stdout and stderr on success. The controlling
terminal receives only the fixed confirmation prompt and escaped secret line.
No success label, item selector, field name, or revision is duplicated through
ordinary process output.

## 7. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly the canonical item plus closed field selectors and
   optional leading named target;
2. current revision selection stays inside the application;
3. refusal and confirmation-input failure publish `Denied` before their
   errors and release no value;
4. wrong field/schema, missing, tombstoned, and conflicted selections publish
   `Failed` without a value;
5. success publishes `Succeeded` with the exact current revision before host
   delivery;
6. audit rows contain no field name, record metadata, confirmation answer, or
   secret bytes;
7. native terminal rendering quotes controls and wipes temporary buffers;
8. the real PTY executable receives the secret only on `/dev/tty`, while
   captured stdout remains empty and restart verification sees one event; and
9. formatting, Clippy, rustdoc, application/host/CLI tests, and downstream
   executable tests pass.
