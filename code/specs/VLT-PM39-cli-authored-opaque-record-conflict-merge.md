# VLT-PM39 — Audited Authored Opaque-Record Conflict Merge

## Status

Normative Phase 1A contract for resolving one current opaque-record conflict
with a complete user-authored canonical-CBOR payload. This is the seventh and
last authored conflict-merge ceremony required by
`VLT-PM00-local-first-password-manager.md` section 23, closing item
9b-3b-2b-2b-6.

## 1. Command and boundary

```text
vault-pm [--vault NAME] conflict merge opaque ITEM BASE_REVISION
```

The exact current live opaque base supplies immutable identity/creation time,
the record's content type, and the favorite, collection, tag, and attachment
state not editable by the Phase 1A form. The controlling terminal collects one
field: the complete replacement payload, as lowercase hexadecimal of its
canonical CBOR encoding, through a hidden prompt. The merge command never
prefills candidate values, accepts inline fields, or chooses a winner. Choosing
one existing candidate unchanged remains
`VLT-PM24-cli-conflict-resolution.md`'s `conflict choose ITEM REVISION`, which
is already schema-agnostic and therefore already covers opaque records.

## 2. What "authoring a merge" means for a record with no schema

The prior six ceremonies each author a fixed field list because this product
knows the record's schema. An opaque record is by definition the one this
product does not know: `decode_record` returns `AnyRecord::Opaque` exactly when
the wire content type is not one of the six first-party types, and it carries
only the content-type string and the canonical-CBOR re-encoding of the inner
payload. There is no title, no secret field, and no optional field to prompt
for, because the field set is whatever the producing client wrote.

Three consequences fix the shape of this ceremony, and each is forced by code
that already exists rather than chosen for convenience.

**The content type is inherited, not authored.** `ItemDocument::validate`
requires the record's own content type to equal the document schema, the merge
base precondition requires every retained live candidate to share the base's
schema, and `merge_item_conflict` re-checks that the authored document's schema
equals every live candidate's. An item's schema is therefore immutable across
its whole history, and the merged record's content type can only be the base's.
This is the exact opposite extreme from
`VLT-PM38-cli-authored-totp-conflict-merge.md`, where every schema field was
authored and nothing was inherited: here exactly one thing is authored and
everything else, including the type of the thing authored, is inherited.

**The payload is one value, so the form is one prompt.** The authored unit is
the whole canonical-CBOR payload. A per-field form is impossible without a
schema, and a partial edit is impossible without a parse. Authoring an opaque
merge means supplying the complete replacement payload, exactly as authoring a
secure-note merge means supplying the complete replacement body.

**The prompt is hidden.** `vault-records` reports `secret_field_count: 0` for an
opaque record, but that means "no field this crate can *name* as secret", not
"no secret". `VLT-PM03`'s `RedactedRecordView::Opaque` already withholds the
entire payload behind the same `RedactedSecret` marker it uses for passwords and
seeds, and reports only its byte length. Fail-closed treatment of an unknown
schema requires assuming the payload is secret-bearing in full, so it is
collected without terminal echo and held in wipe-on-drop ownership like any
other secret in this product.

The rejected alternative was a merge that re-publishes the base payload verbatim
with all current candidates as parents. That is not authoring at all, and
`conflict choose ITEM REVISION` already produces exactly that outcome for any
schema, opaque included. Shipping it again under a second name would add a
ceremony with no capability.

Phase 1A deliberately ships no reveal selector for an opaque payload:
`VLT-PM25-cli-secret-reveal.md`'s `SecretFieldV1` is a closed set of named
first-party fields and gains no opaque member here. An opaque merge is therefore
authored without any way to read the prior payload back through this product,
which is a real limitation and is recorded as one. It does not weaken the
ceremony — every prior candidate stays immutable in history, and a client that
understands the content type can still read it.

## 3. Closed form validation

The host bounds the payload line to its fixed hidden-secret ceiling. The
application restates and extends that bound behind the audited boundary:

- the line is 1–1,024 characters, which at two characters per byte is the same
  512-byte payload ceiling the host's 1,024-byte secret line can carry;
- its length is even and every character is `0-9` or lowercase `a-f`;
- the bytes it spells decode as CBOR with no trailing bytes; and
- re-encoding that CBOR reproduces the typed bytes exactly.

Hexadecimal needs no re-encode comparison of its own: once lowercase and even
length are required, one byte string has exactly one spelling by construction,
unlike the Base32 of VLT-PM38 where unused trailing bits admit several. The
re-encode comparison is applied one level down instead, to the CBOR the bytes
spell, so that a payload accepted here is byte-stable through the storage codec
that will later re-encode it. The comparison is performed by wrapping the typed
payload in the base's content type with `encode_opaque`, decoding the result
with `decode_record`, and requiring the returned payload bytes and content type
back unchanged. A result that is not `AnyRecord::Opaque` is refused as an
internal invariant failure, which makes it structurally impossible to author a
first-party record through this command.

The rules are restated inside the opaque preparation rather than trusted from
the host, so every invalid complete form publishes its failed audit event before
the closed error returns — the same defense-in-depth placement the payment-card,
API-key, database-credential, and TOTP merges use. Phase 1A intentionally
performs no schema interpretation, field extraction, migration, or validation of
the payload's meaning, and makes no claim that the value is accepted by whatever
client produced the content type.

## 4. Opaque preparation and audit ordering

Time and audit-failure randomness are reserved before authentication. The
application consumes the unlocked session and requires an active audit epoch,
at least two current candidates, exact current membership of `BASE_REVISION`, a
live item-bound opaque base, and compatible identity/schema/creation time across
every retained live candidate.

A ready opaque preparation owns the complete wipe-on-drop base without returning
it to the CLI. Missing, unconflicted, noncurrent, tombstone, cross-item, and
first-party-schema bases publish failed item-scoped `ItemConflictMerge` events
before their closed error. Prompt, form-validation, or mutation-entropy failure
consumes the preparation and publishes the same failure before the host error.
Stale pins fail closed; ambiguous publication retains the exact journal.

Success replaces the complete opaque payload, preserves the base content type
and non-form metadata, names the entire former current set as direct causal
parents, and publishes a succeeded `ItemConflictMerge` atomically. Because the
result is authored, its event intentionally omits selected revision. Events
contain no base/candidate identity, content type, payload text or bytes, payload
length or prefix, prompt progress, provider detail, or arbitrary error.

## 5. Secret ownership, output, and storage neutrality

Success emits only `Conflict merged: ITEM`; failure has empty stdout. The
payload is a hidden terminal input and stays in wipe-on-drop ownership until
sealed. The hex decoder accumulates into a wipe-on-drop buffer sized so it
cannot reallocate and wipes its partial nibble on every exit, the canonicality
round trip holds its intermediate wire bytes and decoded record in wipe-on-drop
ownership, and the decoded record is wiped on every path including the refused
non-opaque one, so a rejected line leaves no intact plaintext or partially
decoded payload behind. No opaque value enters arguments, stdout/stderr, audit
history, debug output, config, or durable plaintext. Repository and local-state
access remain injected and provider neutral.

## 6. Acceptance gates

Tests must prove exact default/named grammar; audited missing, unconflicted,
noncurrent, tombstone, first-party-schema, prompt, validation, and entropy
failures; one all-current-parent success that preserves the base content type,
metadata, and immutable history; restart-backed redacted observation; payload
exclusion; named-target isolation; formatting, Clippy, rustdoc,
application/CLI/host tests; and a real executable PTY failure journey that stops
before the authored payload prompt when the target is not a conflict.
