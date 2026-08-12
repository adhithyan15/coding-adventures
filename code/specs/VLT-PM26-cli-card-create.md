# VLT-PM26 — Audited Payment-Card Creation

## Status

Normative Phase 1A contract for authoring and observing one first-party payment
card through the local CLI without disclosing its sensitive fields.

## 1. Purpose and boundary

The local CLI already persists typed encrypted records, publishes every
authenticated create attempt, and can reveal card fields through the separate
VLT-PM25 ceremony. This slice composes the existing first-party `CARD_V1`
record without introducing another storage, crypto, or audit implementation:

```text
vault-pm [--vault NAME] item add card
```

Card editing, import, multiple cards in one command, magnetic-stripe data,
PINs, arbitrary custom fields, and non-interactive input remain outside this
command.

## 2. Closed grammar and prompts

The command accepts no card data, path, output destination, provider option,
or validation bypass in arguments, environment variables, standard input, or
configuration. After the ordinary one-shot unlock it collects exactly these
fixed controlling-terminal prompts in order:

```text
Title:
Cardholder:
Card number:
Expiry month (1-12):
Expiry year (YYYY):
CVV:
Billing postal code (optional):
```

Title and cardholder are required bounded UTF-8 metadata. Expiry month is
canonical ASCII decimal `1` through `12`; year is exactly four ASCII digits
and must be nonzero. Billing postal code is optional bounded UTF-8 metadata.
The host rejects controls and oversized lines before returning a value.

PAN and CVV use echo-disabled, bounded, wipe-on-drop input. PAN is 8 through 19
ASCII digits. CVV is 3 or 4 ASCII digits. The CLI does not claim issuer,
network, checksum, expiration, or authorization validity; those policies are
not universal and must not make offline vault storage dependent on a payment
service.

## 3. Audit-first creation

Before authentication the CLI reserves advisory time, item identity, mutation
randomness, operation identity, audit trace, audit publication randomness, and
failure-event randomness. After successful authentication, every prompt,
terminal, UTF-8 conversion, PAN/CVV/date validation, document encoding, and
repository failure either:

- durably publishes one item-scoped `ItemCreate Failed` event before returning
  its stable payload-free error; or
- atomically publishes the encrypted card revision and one item-scoped
  `ItemCreate Succeeded` event before returning the canonical item selector.

Wrong-passphrase and pre-authentication time/entropy failures publish no item
attempt because no authenticated card access occurred. Retry uses fresh
identity and randomness; ambiguous publication recovery remains the existing
exact journal path.

Audit events contain no title, holder, PAN, PAN suffix, expiry, CVV, postal
code, schema, input length, prompt index, provider detail, path, or arbitrary
error text. They contain only the existing action/outcome, item identity,
trace, actor/device, time, and causal audit fields.

## 4. Secret ownership and redacted observation

Every collected string remains wipe-on-drop until moved into the zeroizing
typed `Card`; document encoding and application publication retain the existing
wipe boundaries. PAN and CVV never enter argv, stdin, environment variables,
configuration, logs, audit metadata, ordinary success output, or a debug
representation.

Authenticated list/search/history surfaces reuse the existing title-only
projection. `item show ITEM` renders only:

```text
Title: "..."
Cardholder: "..."
Last four: "..."
Expiry: MM/YYYY
Card number: <redacted>
CVV: <redacted>
Billing postal code: present|absent
```

The full PAN, CVV, and postal value never enter the redacted view. Explicit PAN
or CVV access requires VLT-PM25 `item reveal` and its separate exact-`yes`,
publish-before-release audit ceremony.

## 5. Errors and output

- malformed grammar, invalid metadata, PAN/CVV/date validation, or document
  validation: invalid;
- wrong passphrase: locked;
- terminal, time, entropy, storage, or audit publication unavailable: provider;
- authenticated corruption: integrity.

Success returns only `Item added: ITEM`. Failure returns only the existing
stable error class. No card field is repeated in ordinary output.

## 6. Acceptance gates

The slice is complete only when tests prove:

1. grammar accepts exactly `item add card`, including command-scoped named
   targets, and rejects extra or secret-bearing arguments;
2. fixed prompt bounds and echo policy classify only PAN and CVV as secrets;
3. host failure and every CLI validation family publish `ItemCreate Failed`
   before returning without a card revision;
4. successful creation publishes one exact `ItemCreate Succeeded` and survives
   a fresh process;
5. list, show, audit, and debug surfaces contain neither PAN nor CVV, audit rows
   admit only the closed event fields, and a filesystem scan excludes the
   collision-resistant full PAN while show exposes only the documented
   redacted metadata;
6. existing audited `item reveal` returns PAN and CVV only through the direct
   terminal channel after separate authorization; and
7. formatting, Clippy, rustdoc, host/CLI tests, and the real PTY executable
   suite pass on the affected dependency closure.
