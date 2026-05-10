# CBR01 — Canonical CBOR Codec

## Overview

A from-scratch implementation of [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949)
in the **deterministic / canonical** profile of §4.2.3 — "Length-
First Map Key Ordering," the profile used by COSE (RFC 9052), CTAP2
(FIDO2), and WebAuthn.

Lives at the same layer as other format primitives. Consumed by
VLT02 (`vault-records`), VLT05 (`vault-auth` for COSE-Key), and
VLT09 (audit log entries).

This document specifies the supported value types, the canonical
profile, the public API, and what is deliberately out of scope. The
implementation lives at `code/packages/rust/canonical-cbor/`.

## Why this primitive exists

Vault records need a wire format where **one logical value maps to
exactly one byte sequence**. Plain CBOR (and plain JSON) does not
have that property:

- The integer 0 has five legal CBOR encodings (`0x00`, `0x18 0x00`,
  …, `0x1B 00 00 00 00 00 00 00 00`).
- `{"a":1,"b":2}` and `{"b":2,"a":1}` are both legal CBOR maps for
  the same logical map.
- Lengths can be definite or indefinite (with `0xFF` break markers).

Vault layers above us assume stable bytes:

1. **AEAD AAD binding** in VLT01 ties a ciphertext to its
   `(namespace, key)` slot. If the same logical record can re-encode
   to different bytes, the tag stops verifying.
2. **Sync conflict detection** in VLT10 compares revision bytes
   directly. Floating encoding kills this.
3. **COSE-Key** in VLT05's WebAuthn / FIDO2-PRF flows is a
   canonical-CBOR-derived format that other implementations refuse
   to consume if our bytes don't match the profile exactly.
4. **Audit log hash chains** in VLT09 reproduce only if the bytes
   are stable across re-encodes.

## Supported value types

```rust
pub enum CborValue {
    Unsigned(u64),        // major type 0
    Negative(u64),        // major type 1; encodes -1 - n
    Bytes(Vec<u8>),       // major type 2
    Text(String),         // major type 3 (UTF-8 validated)
    Array(Vec<CborValue>),// major type 4
    Map(Vec<(CborValue, CborValue)>),  // major type 5
    Tag(u64, Box<CborValue>),          // major type 6
    Bool(bool),           // major type 7 / 20|21
    Null,                 // major type 7 / 22
}
```

Out of scope in v1:

- **Floats.** Vault records do not need them; supporting them
  correctly requires the "shortest of half/single/double that
  preserves the value" rule (RFC 8949 §4.2.2). We will add this when
  a downstream layer needs it.
- **`undefined`** (simple value 23). RFC 8949 explicitly discourages
  it in deterministic encodings. We use `Null` for "absent."

## Canonical profile (RFC 8949 §4.2.3)

Encoder produces, decoder enforces:

| Rule                                                           | Encoder                                          | Decoder rejects                                                   |
|----------------------------------------------------------------|--------------------------------------------------|--------------------------------------------------------------------|
| **Definite length only**                                       | Always definite                                  | Indefinite-length items (info 31; bytes `0x9F`/`0xBF`/`0x5F`/`0x7F`/`0xFF` break) — `Indefinite` |
| **Smallest-form integer encoding**                             | Picks shortest of inline / 1B / 2B / 4B / 8B     | "Expanded" forms (e.g. `0x18 0x05` for 5) — `NonMinimalInteger`   |
| **Length-first map keys**                                      | Sorts encoded keys length-first then bytewise lex | Out-of-order keys, duplicate keys — `NonCanonicalMapOrder`         |
| **Reserved additional-info values 28–30 are illegal**          | Never emits                                      | `Reserved`                                                         |
| **Major type 3 is UTF-8**                                      | `String` is UTF-8 by definition                  | Invalid UTF-8 — `InvalidUtf8`                                      |
| **Only `true`/`false`/`null` simple values**                   | Emits 0xF4/0xF5/0xF6                             | `undefined`, unassigned simple — `UnsupportedSimple`               |
| **No floats**                                                  | Never emits                                      | `0xF9`/`0xFA`/`0xFB` — `FloatNotSupported`                          |
| **One item per `decode` call; no trailing bytes**              | Single item from `encode`                        | Trailing bytes — `TrailingBytes`                                   |
| **Errors source from literals only**                           | n/a                                              | All `Display` strings start with `"canonical-cbor:"` and contain no input bytes |

## Public API

```rust
pub fn encode(v: &CborValue) -> Vec<u8>;
pub fn encode_into(v: &CborValue, out: &mut Vec<u8>);
pub fn decode(bytes: &[u8]) -> Result<CborValue, CborError>;

pub enum CborError {
    UnexpectedEof,
    TrailingBytes,
    Reserved,
    Indefinite,
    NonMinimalInteger,
    InvalidUtf8,
    NonCanonicalMapOrder,
    UnsupportedSimple,
    FloatNotSupported,
}
```

`Display` strings for every variant start with the literal prefix
`"canonical-cbor:"` (asserted by a test).

## Length-first ordering, in detail

Given a map with entries `[(k₁, v₁), …, (kₙ, vₙ)]`:

1. Compute `enc(kᵢ)` for each key (canonical CBOR encoding of the
   key value).
2. Sort the entries by `enc(kᵢ)`:
   - First by `enc(kᵢ).len()` ascending.
   - Ties broken by bytewise lex of `enc(kᵢ)`.
3. Emit `header(major=5, len=n)` then `enc(kᵢ) ‖ enc(vᵢ)` for each
   entry in sorted order.

The decoder verifies that every consecutive pair `(prev_key,
cur_key)` satisfies `prev_key ≺ cur_key` strictly under the same
ordering — this catches both "out of order" and "duplicate key"
with one comparison.

### Why length-first and not bytewise (§4.2.1)?

§4.2.1 (the modern bytewise-only ordering) is preferred for new
applications, but **COSE-Key and CTAP2 require §4.2.3** length-first.
Since vault auth (VLT05) needs to interoperate with FIDO2 / WebAuthn
authenticators that consume COSE-Key, we pick §4.2.3 as the default.
A future v2 may add a bytewise-only mode as a flag.

## Threat model & validation

| Threat                                                         | Defence                                                  | Test                                       |
|----------------------------------------------------------------|----------------------------------------------------------|---------------------------------------------|
| Adversary submits non-canonical bytes hoping for collision     | Strict canonical decoder rejects                         | All `decode_rejects_*` tests (10+)          |
| Adversary submits oversized integer encoding                   | `NonMinimalInteger`                                      | `decode_rejects_one_byte_form_holding_small_value` and similar |
| Adversary submits indefinite-length item                       | `Indefinite`                                             | `decode_rejects_indefinite_array`/`_map`    |
| Adversary submits invalid UTF-8 in text strings                | `InvalidUtf8`                                            | `decode_rejects_invalid_utf8`               |
| Adversary submits unsorted or duplicate map keys               | `NonCanonicalMapOrder`                                   | `decode_rejects_non_canonical_map_order`, `_duplicate_map_keys` |
| Error message injects attacker-controlled bytes into logs      | `Display` strings are static literals only               | `error_messages_are_static`                 |
| Adversary submits trailing bytes after a valid item            | `TrailingBytes`                                          | `decode_rejects_trailing_bytes`             |
| Adversary submits truncated input                              | `UnexpectedEof`                                          | `decode_rejects_eof_in_*`                   |

## Specification non-goals

- We do not implement floats. Add later if needed.
- We do not implement streaming encode/decode. Vault records are
  small; large blobs go through VLT14 which has its own framing.
- We do not implement the full CBOR diagnostic notation parser.
- We do not interpret tag semantics (e.g. tag 0 = ISO-8601 datetime
  string). Tags pass through as `Tag(n, inner)`.

## Citations

- RFC 8949 — *Concise Binary Object Representation (CBOR)*. The
  encoder enforces §4.2 deterministic encoding rules and the decoder
  rejects non-conforming inputs.
- §4.2.3 specifies the length-first map ordering this crate uses.
- RFC 9052 (COSE) §1.4 references this ordering.
- FIDO2 CTAP 2.1 §6 specifies the same ordering for authenticator
  messages.
