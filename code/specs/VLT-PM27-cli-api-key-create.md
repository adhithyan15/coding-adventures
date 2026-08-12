# VLT-PM27 — Audited API-Key Creation

## Status

Normative Phase 1A contract for authoring and observing one first-party API key
through the local CLI without disclosing its token.

## 1. Purpose and boundary

The typed `API_KEY_V1` record, storage-neutral item-create journal, redacted
view, and audited token reveal already exist. This slice composes them into one
closed local command:

```text
vault-pm [--vault NAME] item add api-key
```

Token verification, online service discovery, rotation, revocation, automatic
scope lookup, custom headers, multiline tokens, and non-interactive input are
outside this command.

## 2. Closed grammar and prompts

The command accepts no record field, secret, path, provider option, or bypass
through arguments, environment variables, standard input, URLs, or
configuration. After one-shot unlock it collects these fixed prompts in order:

```text
Label:
Service:
Token:
Scopes (comma-separated, optional):
Expiry Unix seconds (optional):
```

Label and service are required bounded UTF-8 metadata. Token is a required
echo-disabled, bounded, wipe-on-drop UTF-8 secret. Scope input is optional. A
non-empty scope list is split only on commas; the complete scope line is at
most 2,048 UTF-8 bytes, and every component must already be trimmed, non-empty,
unique, and at most 256 UTF-8 bytes, with at most 64 components. The original
order is retained. Expiry is either empty or one canonical nonzero unsigned
decimal integer with no sign or leading zero.

The CLI makes no online claim that the service, scopes, token, or expiry are
currently valid. Offline storage remains independent from issuer APIs.

## 3. Audit-first creation

Before authentication the CLI reserves advisory time, item identity, mutation
randomness, operation identity, audit trace/publication randomness, and
failure-event randomness. After successful authentication, every prompt,
terminal, UTF-8 conversion, scope/expiry validation, document encoding, and
repository failure either:

- durably publishes one item-scoped `ItemCreate Failed` before returning its
  stable payload-free error; or
- atomically publishes the encrypted record and one item-scoped
  `ItemCreate Succeeded` before returning the canonical item selector.

Wrong-passphrase and pre-authentication time/entropy failures do not claim an
authenticated item attempt. Retry uses fresh identities and the existing exact
ambiguous-publication journal.

Audit events contain no label, service, token, token length/prefix, scope,
expiry, schema, prompt index, provider detail, path, or arbitrary error text.

## 4. Secret ownership and redacted observation

Collected strings remain wipe-on-drop until moved into the zeroizing typed
record. Token bytes never enter argv, stdin, environment variables,
configuration, ordinary CLI output, logs, audit metadata, or debug output.

`item show ITEM` renders only:

```text
Label: "..."
Service: "..."
Scope: "..."        # repeated, or `Scopes: none`
Expiry: UNIX_SECONDS # or `Expiry: none`
Token: <redacted>
```

List/search/history continue to use the existing label-only projection.
Explicit token access requires `item reveal ITEM api-key-token` and the
separate VLT-PM25 exact-`yes`, publish-before-release terminal ceremony.

## 5. Errors and output

- malformed grammar or metadata/scope/expiry/document validation: invalid;
- wrong passphrase: locked;
- terminal, time, entropy, storage, or audit publication unavailable: provider;
- authenticated corruption: integrity.

Success returns only `Item added: ITEM`. Failure returns only the existing
stable error class.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `item add api-key`, including command-scoped named
   targets, and rejects extra or secret-bearing arguments;
2. only the token prompt is hidden while metadata prompts are bounded;
3. host, UTF-8, scope, expiry, and document failures durably publish
   `ItemCreate Failed` before returning and create no record;
4. success publishes exactly one `ItemCreate Succeeded` and survives restart;
5. list/show/audit/debug exclude token bytes, show contains only the documented
   metadata, and audit rows admit only the closed event fields;
6. the full collision-resistant token is absent from the persisted profile;
7. existing audited reveal returns the token only through direct terminal
   delivery after separate authorization; and
8. formatting, Clippy, rustdoc, host/CLI tests, and real PTY executable tests
   pass on the affected dependency closure.
