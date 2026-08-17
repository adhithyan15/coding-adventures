# VLT-PM38 — Audited Authored TOTP Conflict Merge

## Status

Normative Phase 1A contract for resolving one current TOTP-seed conflict with a
complete user-authored TOTP record.

## 1. Command and boundary

```text
vault-pm [--vault NAME] conflict merge totp ITEM BASE_REVISION
```

The exact current live TOTP base supplies immutable identity/creation time and
the favorite, collection, tag, and attachment state not editable by the Phase 1A
form. The controlling terminal collects a complete label, optional issuer,
hidden Base32 seed, algorithm, digit count, and period. Existing seeds are
inspected only through `conflict reveal ITEM REVISION totp-secret` and its
separate `VLT-PM25-cli-secret-reveal.md` authorization; the merge command never
prefills candidate values, accepts inline fields, or chooses a winner. Other
schemas remain separate backlog ceremonies.

Unlike the database credential of
`VLT-PM37-cli-authored-database-credential-conflict-merge.md`, `TOTP_SEED_V1`
carries no dynamic or issuance-only attribute: label, issuer, seed, algorithm,
digits, and period are the whole schema and all six are authored here. There is
therefore nothing for a merged record to inherit from the base candidate beyond
the non-form metadata above, and no field resets to a static default. QR
scanning, `otpauth://` parsing, code generation, HOTP counters, and clock
correction stay outside this command exactly as in
`VLT-PM29-cli-totp-create.md`.

## 2. Closed form validation

The host bounds every field before application entry. The application also
requires the same closed parameter rules VLT-PM29 create already applies: an
algorithm of exactly `SHA1`, `SHA256`, or `SHA512`; a digit count of exactly `6`
or `8`; a period that is one canonical decimal integer in `1..=3600` with no
sign and no leading zero; and a seed that is canonical unpadded RFC 4648 Base32
of 1–256 characters drawn only from `A-Z2-7`, with zero unused trailing bits,
decoding to 1–160 bytes.

The rules are restated inside the opaque preparation rather than trusted from
the host, so every invalid complete form publishes its failed audit event before
the closed error returns — the same defense-in-depth placement the payment-card,
API-key, and database-credential merges use. Canonicality is decided by
re-encoding the decoded bytes and requiring the exact typed line back, so one
stored seed always has exactly one accepted spelling. Phase 1A intentionally
performs no code generation, issuer lookup, network call, or clock check, and
makes no claim that the seed or parameters are accepted by any issuer.

## 3. Opaque preparation and audit ordering

Time and audit-failure randomness are reserved before authentication. The
application consumes the unlocked session and requires an active audit epoch,
at least two current candidates, exact current membership of `BASE_REVISION`, a
live item-bound TOTP base, and compatible identity/schema/creation time across
every retained live candidate.

A ready opaque preparation owns the complete wipe-on-drop base without returning
it to the CLI. Missing, unconflicted, noncurrent, tombstone, cross-item, and
wrong-schema bases publish failed item-scoped `ItemConflictMerge` events before
their closed error. Prompt, form-validation, or mutation-entropy failure
consumes the preparation and publishes the same failure before the host error.
Stale pins fail closed; ambiguous publication retains the exact journal.

Success replaces the complete TOTP payload, preserves base non-form metadata,
names the entire former current set as direct causal parents, and publishes a
succeeded `ItemConflictMerge` atomically. Because the result is authored, its
event intentionally omits selected revision. Events contain no base/candidate
identity, label, issuer, seed text or bytes, seed length or prefix, algorithm,
digits, period, prompt progress, provider detail, or arbitrary error.

## 4. Secret ownership, output, and storage neutrality

Success emits only `Conflict merged: ITEM`; failure has empty stdout. The seed
is a hidden terminal input and every authored field stays in wipe-on-drop
ownership until sealed. The decoder accumulates into a wipe-on-drop buffer and
wipes its partial bit accumulator on every exit, so a rejected line leaves no
intact plaintext or partially decoded seed behind. No TOTP value enters
arguments, stdout/stderr, audit history, debug output, config, or durable
plaintext. Repository and local-state access remain injected and provider
neutral.

## 5. Acceptance gates

Tests must prove exact default/named grammar; audited missing, unconflicted,
noncurrent, tombstone, wrong-schema, prompt, validation, and entropy failures;
one all-current-parent success that preserves base metadata and immutable
history; restart-backed redacted observation; seed exclusion; named-target
isolation; formatting, Clippy, rustdoc, application/CLI tests; and a real
executable PTY failure journey that stops before the authored form when the
target is not a conflict.
