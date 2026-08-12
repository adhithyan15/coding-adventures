# VLT-PM29 — Audited TOTP Creation

## Status

Normative Phase 1A contract for authoring and observing one first-party TOTP
seed through the local CLI without disclosing its shared secret by default.

## 1. Purpose and boundary

The typed `TOTP_SEED_V1` record, storage-neutral item-create journal, redacted
view, and audited raw-secret selector already exist. This slice composes them
into one closed local command:

```text
vault-pm [--vault NAME] item add totp
```

QR scanning, `otpauth://` parsing, code generation/display, HOTP counters,
online issuer discovery, clock correction, migration formats, and
non-interactive input are outside this command.

## 2. Closed grammar and prompts

The command accepts no record field, secret, path, provider option, or bypass
through arguments, environment variables, standard input, URLs, or
configuration. After one-shot unlock it collects these fixed prompts in order:

```text
Label:
Issuer (optional):
Secret (Base32):
Algorithm (SHA1/SHA256/SHA512):
Digits (6 or 8):
Period seconds (1-3600):
```

Label is required control-free UTF-8 metadata of at most 256 bytes; issuer is
optional under the same bound. Secret input is echo-disabled, wipe-on-drop,
and must be canonical unpadded RFC 4648 Base32: 1–256 uppercase ASCII
characters drawn only from `A-Z2-7`, with zero unused trailing bits, decoding
to 1–160 bytes. Algorithm is exactly `SHA1`, `SHA256`, or `SHA512`; digits is
exactly `6` or `8`; period is one canonical decimal integer in `1..=3600`
with no sign or leading zero.

The CLI makes no online claim that the issuer accepts the seed or parameters.
The decoded encrypted record remains independent from authenticator and
storage providers.

## 3. Audit-first creation

Before authentication the CLI reserves advisory time, item identity, mutation
randomness, operation identity, audit trace/publication randomness, and
failure-event randomness. After successful authentication, every prompt,
terminal, UTF-8 conversion, Base32/parameter validation, document encoding,
and repository failure either:

- durably publishes one item-scoped `ItemCreate Failed` before returning its
  stable payload-free error; or
- atomically publishes the encrypted record and one item-scoped
  `ItemCreate Succeeded` before returning the canonical item selector.

Wrong-passphrase and pre-authentication time/entropy failures do not claim an
authenticated item attempt. Retry uses fresh identities and the existing exact
ambiguous-publication journal.

Audit events contain no label, issuer, secret, secret length/prefix, algorithm,
digits, period, schema, prompt index, provider detail, path, or arbitrary error
text.

## 4. Secret ownership, observation, and disclosure

Collected strings remain wipe-on-drop until metadata moves into the zeroizing
typed record and the decoded secret bytes move into its secret field. Secret
text or bytes never enter argv, stdin, environment variables, configuration,
ordinary CLI output, logs, audit metadata, or debug output.

`item show ITEM` renders only:

```text
Label: "..."
Issuer: "..." # or `Issuer: none`
Algorithm: SHA1
Digits: 6
Period: 30
Secret: <redacted>
```

List/search/history continue to use the existing label-only projection.
Explicit seed access requires `item reveal ITEM totp-secret` and the separate
VLT-PM25 exact-`yes`, publish-before-release terminal ceremony. Only after the
successful audit publication does the CLI encode the selected raw bytes as
canonical unpadded Base32 in a wipe-on-drop buffer and deliver them directly to
the terminal. Raw bytes are never written.

## 5. Errors and output

- malformed grammar or metadata/Base32/parameter/document validation: invalid;
- wrong passphrase: locked;
- terminal, time, entropy, storage, or audit publication unavailable: provider;
- authenticated corruption: integrity.

Success returns only `Item added: ITEM`. Failure returns only the existing
stable error class.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `item add totp`, including command-scoped named
   targets, and rejects extra or secret-bearing arguments;
2. only the Base32 secret prompt is hidden while every metadata prompt is
   bounded;
3. Base32 round trips canonical boundary vectors and rejects lowercase,
   padding, impossible lengths, nonzero trailing bits, emptiness, and overflow;
4. host, UTF-8, seed, parameter, and document failures durably publish
   `ItemCreate Failed` before returning and create no record;
5. success publishes exactly one `ItemCreate Succeeded` and survives restart;
6. list/show/audit/debug exclude secret text and bytes, show contains only the
   documented metadata, and audit rows admit only the closed fields;
7. the full collision-resistant seed is absent from the persisted profile;
8. audited reveal returns canonical Base32 only through direct terminal
   delivery after separate authorization; and
9. formatting, Clippy, rustdoc, host/CLI tests, and real PTY executable tests
   pass on the affected dependency closure.
