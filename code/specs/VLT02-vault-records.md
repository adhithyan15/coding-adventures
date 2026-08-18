# VLT02 — Typed Vault Records

## Overview

The **typed-record layer** of the Vault stack. Sits *above*
[`canonical-cbor`](./CBR01-canonical-cbor.md) and *below*
[`vault-sealed-store`](./VLT01-vault-sealed-store.md): each record
is a typed Rust struct that codecs to canonical CBOR bytes which are
then wrapped in envelope encryption (VLT01) and persisted via
`storage-core`.

This document specifies the wire envelope, the `VaultRecord` trait,
the first-party record types, version handling, forward
compatibility, and security properties. Implementation lives at
`code/packages/rust/vault-records/`.

## Why this layer exists

VLT01 stores opaque `Vec<u8>` plaintext. Without a typed record
layer, every application built on the vault would hand-roll the
same struct ↔ bytes serialisation, with the same bugs. VLT02
centralises:

1. **Canonical encoding.** Every record is canonical CBOR — the
   same struct value always produces the same bytes. AEAD AAD
   binding in VLT01 depends on this; sync conflict detection in
   VLT10 depends on this.
2. **Content-type tagging.** Records self-describe with a string
   like `"vault/login/v1"`. Decoders dispatch on it; unknown types
   pass through as opaque so an old client sees a future record
   type as "I don't know what this is" rather than crashing.
3. **A first-party schema set** covering both reference targets
   (Bitwarden / 1Password class **and** HashiCorp Vault / AWS
   Secrets Manager class) on the same primitive.

## Wire envelope

Every encoded record is a CBOR map of exactly two entries:

```text
   record_bytes = canonical_cbor({
       "t": <text>,    // content type, e.g. "vault/login/v1"
       "d": <map>,     // payload — schema-specific fields
   })
```

Why short keys (`"t"` / `"d"`)? Records are stored once per user
(potentially millions per organisation), so two-byte CBOR text-
string headers on every key matter.

Why `"d"` *first* on the wire? Both keys are length-1 text strings,
so the canonical-CBOR length-first ordering is tied; the bytewise
lex tiebreak puts `'d' < 't'`. Deterministic.

Why `t` outside the payload (rather than as a CBOR tag)? Tags in
the canonical-CBOR profile pass through opaquely and are not
interpreted; using them for content-typing would mix structure
and semantics.

## `VaultRecord` trait

```rust
pub trait VaultRecord: Sized {
    const CONTENT_TYPE: &'static str;
    fn encode_payload(&self) -> CborValue;
    fn decode_payload(payload: &CborValue) -> Result<Self, VaultRecordError>;
}
```

Implementors:

- declare `CONTENT_TYPE` (e.g. `"vault/login/v1"`),
- map their fields to a `CborValue::Map` in `encode_payload`,
- recover their fields from a `CborValue::Map` in `decode_payload`,
- implement `Zeroize` (manually) so secret fields wipe on drop.

`encode_record(&T) -> Result<Vec<u8>, VaultRecordError>` and
`decode_record_as::<T>(&[u8]) -> Result<T, …>` wrap the trait into the
full `{t, d}` envelope. `decode_record(&[u8]) -> Result<AnyRecord, …>`
is the content-type-dispatching counterpart. Every one of these four
entry points is fallible in both directions; see *Encoding is
fallible* below for why encoding is not infallible even though the
input is a well-typed Rust struct.

## Encoding is fallible

Encoding a record can fail, and the fallibility is not a formality.
The reason is an asymmetry between two independent ceilings that this
crate sits between:

| Ceiling                                | Owner            | Value  | Applies to                          |
|----------------------------------------|------------------|--------|-------------------------------------|
| `MAX_ENCODED_SIZE`                     | `canonical-cbor` | 1 MiB  | any single CBOR-encoded value       |
| `MAX_ENCODE_DEPTH` / `MAX_DECODE_DEPTH`| `canonical-cbor` | 128    | nesting of any single value         |
| `MAX_PLAINTEXT_BYTES`                  | VLT-PM05         | 16 MiB | one encrypted application object    |

The application's plaintext gate is **sixteen times larger** than the
codec's encoded-size gate. Nothing reconciles them, and nothing should:
they answer different questions. `MAX_PLAINTEXT_BYTES` bounds what the
AEAD layer will seal in one frame; `MAX_ENCODED_SIZE` bounds what the
canonical encoder will ever emit for one value, which is a
denial-of-service bound on the codec itself.

The consequence is a range of record sizes — roughly 1 MiB to 16 MiB —
that are **legal to hold and legal to decode but illegal to encode**.
A record in that range is not hypothetical:

- A peer device whose own framing budget is larger authors a record
  with, say, a 2 MiB `Login.password`. It is well under the 16 MiB
  plaintext gate, so it seals, syncs, and decodes locally without
  complaint.
- The local catalog now holds a record that the encoder will refuse to
  re-emit. Every later operation that re-serialises it — `item edit`,
  any of the seven authored conflict-merge ceremonies (VLT-PM33
  through VLT-PM39), `conflict choose`, `history restore`, and
  `export` — reaches the encoder again.

Had `encode_record` been infallible, that second step would abort the
process rather than return an error. **A process abort is strictly
worse than a closed error**: it discards in-flight state, it violates
VLT-PM00's fail-closed contract, and because the poisoned record
persists in the store, the abort repeats on every subsequent command
against the same vault. That is a local denial of service reachable by
one record from an untrusted peer.

So `encode_record` returns `Result` and reports
`VaultRecordError::Cbor(CborError::EncodeTooLarge)`. Failing closed
loses one record; panicking loses the process.

(The sibling `EncodeTooDeep` is *not* reachable through
`encode_record`. Every first-party payload has a fixed, shallow shape —
the deepest is envelope → payload map → a `Vec<String>` field → text,
four levels against a limit of 128 — so no first-party record can be
nested past the encoder's depth cap. Depth is reachable only through
`encode_opaque`, whose payload is caller-supplied CBOR of arbitrary
shape, and that path is already covered.) This is the same rule and the same remedy already applied
to `encode_opaque` and to `decode_record`'s opaque arm; it is stated
here once because it governs all first-party record types equally, not
just the opaque one.

Two properties follow that callers may rely on:

1. **No partial output.** `try_encode` builds into a local buffer and
   returns `Err` without handing back bytes, so a rejected record never
   produces a truncated encoding that a later reader could mistake for
   a whole one.
2. **The boundary is exact, not approximate.** A record whose full
   envelope encodes to exactly `MAX_ENCODED_SIZE` bytes succeeds; one
   byte more fails. Callers must not assume slack.

One property callers may *not* rely on: the rejected encode does **not**
wipe what it had already serialised. `try_encode` drops its partial
output buffer — up to `MAX_ENCODED_SIZE` of plaintext — without
zeroizing it, so a refused record leaves a copy of its secret fields in
freed heap.

This is not new to the `Result`: the panicking wrapper reached that
state by the same path, since `encode` is `try_encode(...).expect(...)`
and the buffer was already dropped before the panic. What is new is
that the process now survives to keep running with that heap freed but
unwiped.

It is left as it stands, and the reason is that wiping the output
buffer alone would be misleading rather than protective. The same
plaintext is simultaneously held in the `CborValue` tree the caller
built and still owns, and canonical-CBOR's value types implement
neither `Zeroize` nor a wiping `Drop` on any path — the property
VLT-PM39 §5 already records for the whole codec layer. Wiping one copy
while the larger one persists would buy a guarantee the layer cannot
actually make. Closing it properly means making canonical-CBOR
zeroize-aware end to end, which adds a dependency to a deliberately
zero-dependency foundational crate and so belongs to its own change.

## First-party record types

| Type                  | Content type              | Use case                                   |
|-----------------------|---------------------------|--------------------------------------------|
| `Login`               | `vault/login/v1`          | Username + password + URLs + notes         |
| `SecureNote`          | `vault/note/v1`           | Free-form encrypted note                   |
| `Card`                | `vault/card/v1`           | Credit / payment card with validation      |
| `TotpSeed`            | `vault/totp/v1`           | TOTP / HOTP shared secret                  |
| `ApiKey`              | `vault/api-key/v1`        | Static API token + scopes + expiry         |
| `DatabaseCredential`  | `vault/db-credential/v1`  | DB user/pass + host/port + lease metadata  |

`Login` / `SecureNote` / `Card` / `TotpSeed` cover the
password-manager case. `ApiKey` / `DatabaseCredential` cover the
machine-secrets case. The same primitive serves both.

## Versioning

Content types are suffixed `vN`. Schema evolution gets a fresh tag
(`vault/login/v2`). Decoders that only know v1 see v2 records as
`AnyRecord::Opaque` — they don't crash, they just don't see the
new fields. Migration helpers (read v1, return a v2 struct) live
beside the new type.

## Forward compatibility

`decode_payload` impls walk the payload's CBOR map by named field
and tolerate unknown extra fields. This means a v1 client can
read v1.1 records that added optional fields — it sees the
fields it knows and ignores the rest.

Required-but-missing fields fail with `SchemaMismatch { what: "…" }`
where `what` is a `&'static str` picked from a per-key match,
never from input bytes.

## Sensitive material handling

Every type carrying a secret implements `Zeroize` manually:
`Login.password`, `Card.cvv`, `Card.number`, `TotpSeed.secret`,
`ApiKey.token`, `DatabaseCredential.password`. Higher Vault layers
hold records inside `Zeroizing<T>` so drops always wipe.

Every first-party record type also implements a closed, value-redacted
`Debug`. The output contains only the Rust type name and a fixed `<redacted>`
marker. It never formats display metadata, secret fields, list contents, or
optional values. `AnyRecord` applies the same rule while retaining only its
variant name; the opaque variant does not format its attacker-controlled
content type or canonical payload bytes. These guarantees apply even when a
caller bypasses the higher-level VLT-PM03 redacted view API.

## Errors are inert

`VaultRecordError`'s `Display` and `Debug` strings come exclusively from this
crate's literals. Specifically:

- `Cbor` reports the underlying CBOR error variant tag, not bytes.
- `NotARecord` / `BadEnvelope` are constants.
- `ContentTypeMismatch { expected, actual }` carries `actual` for
  callers that want to inspect via pattern matching, but the
  `Display` impl deliberately formats only the `expected`
  literal — so log lines never contain attacker-controlled bytes.
- `SchemaMismatch { what }` uses a `&'static str` from a fixed
  per-field table.

`Debug` exposes only the variant name and suppresses the
attacker-controlled `actual` content type. This keeps diagnostic and assertion
formatting at the same trust boundary as `Display`.

This matches VLT01's discipline.

## Threat model & test coverage

| Threat                                                          | Defence                                                               | Test                                                            |
|-----------------------------------------------------------------|-----------------------------------------------------------------------|-----------------------------------------------------------------|
| Decoder crashes on attacker-crafted CBOR                        | Inherited from canonical-cbor (depth caps, length caps, etc.)         | (covered in `CBR01`)                                            |
| Two distinct byte sequences decode to "the same logical record" | Canonical CBOR strictness + envelope's exact-2-keys check             | `decode_then_reencode_is_byte_stable`, `encode_is_byte_stable`  |
| Confusing one record type for another                           | `decode_record_as` rejects mismatched content type                    | `decode_record_as_rejects_wrong_content_type`                   |
| Old client crashes on new content type                          | `AnyRecord::Opaque` pass-through                                      | `unknown_content_type_decodes_as_opaque`                        |
| Schema validation bypass via missing fields                     | `SchemaMismatch` on missing required fields                           | `login_missing_password_is_schema_mismatch`                     |
| Card with impossible expiry month                               | `expiry_month` validated 1..=12                                       | `card_with_invalid_month_is_schema_mismatch`                    |
| TOTP with absurd digit count                                    | `digits` validated 4..=10                                             | `totp_with_invalid_digits_is_schema_mismatch`                   |
| Top-level value other than the `{t, d}` map                     | `NotARecord`                                                          | `decode_rejects_top_level_array`, `…_with_extra_field`          |
| `"t"` is not text                                               | `BadEnvelope`                                                         | `decode_rejects_envelope_with_t_not_text`                       |
| Attacker-controlled bytes in error messages                     | `Display` strings static literals only; `actual` field hidden         | `error_display_strings_are_static`                              |
| Plaintext record values leak through diagnostic formatting      | Closed custom `Debug` on records and `AnyRecord`                       | `record_debug_is_value_redacted`, `any_record_debug_is_value_redacted` |
| Attacker content type leaks through error diagnostic formatting | Closed custom `Debug` on `VaultRecordError`                            | `error_debug_strings_are_static`                                |
| Forward incompatibility breaks v1 readers                       | Unknown payload fields tolerated                                      | `extra_unknown_fields_in_payload_are_ignored`                   |
| Peer record between the two ceilings aborts the process on re-encode | `encode_record` returns `Result` via `try_encode`                | `encode_record_reports_an_oversized_record_instead_of_panicking` |
| Every first-party type shares that exposure, not just `Login`   | The `Result` is on the generic entry point, above the trait            | `every_first_party_record_type_reports_oversize`                |

## Non-goals

- **No encryption.** That's VLT01.
- **No persistence.** That's `storage-core`.
- **No app-specific record validation** (e.g. URL scheme, EAN-13
  for `Card.number`). The vault library doesn't second-guess apps.
- **No automatic schema migration.** That's a per-version
  concern; version `vN+1` types ship their own migrators when
  needed.

## Citations

- `CBR01-canonical-cbor.md` — the codec this crate sits on.
- `VLT01-vault-sealed-store.md` — the envelope-encryption layer
  that sees these bytes as opaque plaintext.
- `VLT00-vault-roadmap.md` — full Vault layering and the
  reference targets this typed layer enables.
