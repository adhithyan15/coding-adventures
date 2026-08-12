# VLT-PM28 — Audited Database-Credential Creation

## Status

Normative Phase 1A contract for authoring and observing one static database
credential through the local CLI without disclosing its password.

## 1. Purpose and boundary

The typed `DATABASE_CREDENTIAL_V1` record, storage-neutral item-create journal,
redacted view, and audited database-password reveal already exist. This slice
composes them into one closed local command:

```text
vault-pm [--vault NAME] item add database-credential
```

Network connection tests, DNS resolution, driver discovery, TLS configuration,
connection strings, dynamic leases, rotation, and non-interactive input are
outside this command. Locally authored credentials always have no lease ID or
lease expiry; dynamic issuance remains a later VLT08 concern.

## 2. Closed grammar and prompts

The command accepts no field, secret, path, provider option, or bypass through
arguments, environment variables, standard input, URLs, or configuration.
After one-shot unlock it collects these fixed prompts in order:

```text
Label:
Engine:
Host:
Port:
Database (optional):
Username:
Password:
```

Label, host, and username are required control-free UTF-8 metadata of at most
256 bytes. Database is optional under the same bound. Engine is a canonical
provider-neutral identifier of 1–32 ASCII bytes: it starts with a lowercase
letter and continues only with lowercase letters, digits, `-`, or `_`. Port is
one canonical decimal integer in `1..=65535`, with no sign or leading zero.
Password is a required echo-disabled, bounded, wipe-on-drop UTF-8 secret.

The CLI makes no online claim that the engine, endpoint, account, or password
is valid or reachable. The encrypted record remains independent from database
drivers and storage providers.

## 3. Audit-first creation

Before authentication the CLI reserves advisory time, item identity, mutation
randomness, operation identity, audit trace/publication randomness, and
failure-event randomness. After successful authentication, every prompt,
terminal, UTF-8 conversion, engine/port validation, document encoding, and
repository failure either:

- durably publishes one item-scoped `ItemCreate Failed` before returning its
  stable payload-free error; or
- atomically publishes the encrypted record and one item-scoped
  `ItemCreate Succeeded` before returning the canonical item selector.

Wrong-passphrase and pre-authentication time/entropy failures do not claim an
authenticated item attempt. Retry uses fresh identities and the existing exact
ambiguous-publication journal.

Audit events contain no label, engine, host, port, database, username,
password, password length/prefix, schema, prompt index, provider detail, path,
or arbitrary error text.

## 4. Secret ownership and redacted observation

Collected strings remain wipe-on-drop until moved into the zeroizing typed
record. Password bytes never enter argv, stdin, environment variables,
configuration, ordinary CLI output, logs, audit metadata, or debug output.

`item show ITEM` renders only:

```text
Label: "..."
Engine: "..."
Host: "..."
Port: 5432
Database: "..." # or `Database: none`
Username: "..."
Lease: absent
Expiry: none
Password: <redacted>
```

List/search/history continue to use the existing label-only projection.
Explicit password access requires `item reveal ITEM database-password` and the
separate VLT-PM25 exact-`yes`, publish-before-release terminal ceremony.

## 5. Errors and output

- malformed grammar or metadata/engine/port/document validation: invalid;
- wrong passphrase: locked;
- terminal, time, entropy, storage, or audit publication unavailable: provider;
- authenticated corruption: integrity.

Success returns only `Item added: ITEM`. Failure returns only the existing
stable error class.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `item add database-credential`, including
   command-scoped named targets, and rejects extra or secret-bearing arguments;
2. only the password prompt is hidden while every metadata prompt is bounded;
3. host, UTF-8, engine, port, and document failures durably publish
   `ItemCreate Failed` before returning and create no record;
4. success publishes exactly one `ItemCreate Succeeded` and survives restart;
5. list/show/audit/debug exclude password bytes, show contains only the
   documented static metadata, and audit rows admit only the closed fields;
6. the full collision-resistant password is absent from the persisted profile;
7. existing audited reveal returns the password only through direct terminal
   delivery after separate authorization; and
8. formatting, Clippy, rustdoc, host/CLI tests, and real PTY executable tests
   pass on the affected dependency closure.
