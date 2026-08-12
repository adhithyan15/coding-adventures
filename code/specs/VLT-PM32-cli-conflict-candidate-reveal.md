# VLT-PM32 — Audited Conflict-Candidate Secret Reveal

## Status

Normative Phase 1A contract for inspecting one secret field from one exact
current conflict candidate before an authored merge or choose-existing
resolution.

## 1. Purpose and boundary

The local CLI exposes:

```text
vault-pm [--vault NAME] conflict reveal ITEM REVISION FIELD
```

This is an explicit interactive disclosure ceremony. It reveals one supported
secret field from the exact named revision only when that revision is one of at
least two current candidates for the named item. It does not reveal a complete
document, accept a historical-but-noncurrent revision, select a winner, mutate
the conflict, persist plaintext, use stdout, or add clipboard support.

The boundary is storage neutral. Item and revision capabilities are resolved
inside the authenticated application over the injected repository; the CLI
does not inspect provider bytes or derive provider paths.

## 2. Closed grammar

`ITEM` and `REVISION` use their canonical uppercase user encodings. `FIELD` is
one of the closed first-party disclosure selectors:

- `login-password`
- `login-notes`
- `secure-note-body`
- `card-number`
- `card-cvv`
- `api-key-token`
- `database-password`
- `totp-secret`

Missing, extra, lowercase/noncanonical identity, unknown-field, flag, stdin,
environment, and inline-secret forms are grammar errors before authentication.
The command contains capabilities and a field selector but no secret value.

## 3. Exact candidate authorization

After one-shot unlock, the application validates all of the following inside
one consumed session:

1. the item exists in the authenticated current catalog;
2. it has at least two retained current candidates;
3. the exact revision belongs to that current candidate set;
4. the candidate is live rather than a tombstone;
5. its document identity matches the named item; and
6. the requested field belongs to the candidate's authenticated schema.

A reachable historical revision that is no longer current is not a conflict
candidate and must fail. Validation never selects or discards a candidate and
never changes the repository or current catalog.

## 4. Audit-first disclosure ceremony

Time and audit randomness are reserved before authentication. After unlock,
the host asks on the controlling terminal for the same exact-`yes`
confirmation used by current-item reveal. The application then uses a single
publish-before-release `ItemRead` boundary:

- refusal or confirmation-host failure publishes `Denied`, bound to the item
  but not a revision, without traversing or decrypting the candidate;
- missing item, unconflicted item, noncandidate revision, tombstone, item
  mismatch, and schema/field mismatch publish `Failed` before their closed
  error is released;
- once current-candidate membership is authenticated, tombstone or field
  failure may bind the selected revision; and
- success binds both item and exact revision and publishes `Succeeded` before
  the owned secret is released to the CLI.

Audit publication failure supersedes and withholds the denial, original error,
or secret, while retaining the exact recovery journal. Events contain no
field selector, schema, title, candidate metadata, secret bytes, confirmation
text, provider detail, or arbitrary error text.

New CLI vaults already have an audit-first generation-zero chain. This command
is audit-required and does not introduce a legacy unaudited disclosure path.

## 5. Direct terminal delivery

After the succeeded event is durable, UTF-8 secrets are control-escaped and
written directly to the controlling terminal. TOTP bytes use the existing
canonical uppercase unpadded Base32 rendering. Ordinary stdout is exactly
empty; the secret is never returned in `CliOutput`, stderr, `Debug`, audit
history, or storage. The owned disclosed value is non-cloneable and
wipe-on-drop.

The controlling terminal is trusted for this explicit ceremony. Terminal
scrollback and an observer able to view the terminal are outside process
custody and are called out by the existing reveal warning.

## 6. Stable failures

Wrong passphrase returns locked. A missing item or noncandidate revision
returns not-found. An unconflicted item returns conflict-required. Tombstone,
item mismatch, field mismatch, and refusal return the existing closed invalid
class. Repository, audit publication, terminal, and entropy failures use their
existing provider/host classes. Every failure has empty stdout and contains no
secret-bearing payload.

## 7. Acceptance gates

The slice is complete only when tests prove:

1. the parser accepts only canonical `ITEM REVISION FIELD` inputs, including a
   command-scoped named vault;
2. refusal publishes `Denied` without candidate traversal or terminal secret
   delivery;
3. missing, unconflicted, wrong-item, noncandidate, tombstone, and field
   mismatch cases publish `Failed` before their closed errors;
4. success releases exactly the selected candidate field only after a durable
   succeeded event that binds item and revision;
5. the conflict and every immutable candidate remain unchanged after denial,
   failure, and success;
6. UTF-8 and TOTP encodings reuse the current direct-terminal delivery path
   with empty ordinary stdout;
7. audit renderers, errors, process output, and durable storage contain no
   disclosed secret; and
8. formatting, Clippy, rustdoc, application/CLI tests, and relevant executable
   PTY tests pass on the affected dependency closure.
