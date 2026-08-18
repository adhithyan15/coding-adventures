# `coding_adventures_canonical_cbor`

Deterministic CBOR encoder + decoder, from scratch.
[RFC 8949](https://www.rfc-editor.org/rfc/rfc8949) data with the
**§4.2.3 "Length-First Map Key Ordering"** profile (CTAP2 / COSE /
WebAuthn canonical).

## Why deterministic CBOR

JSON has the same problem CBOR does: many byte sequences encode "the
same logical value." Vault records need the *opposite* — one logical
value, one byte sequence. We need that for:

- AEAD authentication (the AAD-bound ciphertext only verifies if the
  bytes match exactly).
- Sync conflict detection (compare revisions byte-for-byte).
- COSE-Key formatting in FIDO2 / WebAuthn-PRF flows (VLT05
  bind-mode authenticators consume canonical CBOR).

This crate encodes the canonical profile by construction and
*rejects non-canonical input on decode*. There is no "permissive
mode."

For untrusted or caller-built values, use `try_encode` or
`try_encode_into`. The checked encoder rejects duplicate encoded map keys,
nesting beyond 128 levels, and output beyond 1 MiB. Its diagnostics are static
and never include payload bytes. The original `encode` and `encode_into`
helpers remain source-compatible convenience wrappers over that checked path.

## Quick example

```rust
use coding_adventures_canonical_cbor::{CborValue, encode, decode};

let v = CborValue::Map(vec![
    (CborValue::text("title"), CborValue::text("hello")),
    (CborValue::text("count"), CborValue::Unsigned(42)),
]);
let bytes = encode(&v);
// Map entries are reordered length-first / bytewise-lex at encode
// time, so passing them in any order is fine — the bytes are stable.

let back = decode(&bytes).unwrap();
assert_eq!(encode(&back), bytes); // round-trip is byte-for-byte stable
```

## Passing a sub-document through, without re-encoding it

`decode_map_spanned` decodes a map and tells you where each entry's value
sat in the input:

```rust
use coding_adventures_canonical_cbor::{CborValue, decode_map_spanned, encode};

let bytes = encode(&CborValue::Map(vec![
    (CborValue::text("d"), CborValue::bytes(vec![1, 2, 3])),
    (CborValue::text("t"), CborValue::text("example/v1")),
]));
let entries = decode_map_spanned(&bytes).unwrap().unwrap();
assert_eq!(
    &bytes[entries[0].value_span.clone()],
    encode(&CborValue::bytes(vec![1, 2, 3])).as_slice(),
);
```

Those bytes are exactly what re-encoding the value would produce. The
decoder enforces every rule the encoder applies, so an input that decodes
has only one legal spelling — the one the encoder emits.

Which raises the obvious question: why not just re-encode? Because
`decode` has no input-length bound and `try_encode` has a 1 MiB one. That
bound protects the *encoder*; it says nothing about what may be read. So
there are inputs that decode and will not re-encode, and for those a
round trip turns a readable document into an error. If your job is to
hold someone else's sub-document and hand it back later without
interpreting it, take the span: slicing a range the parser itself
measured cannot fail on anything that decoded.

Valid CBOR that isn't a map is `Ok(None)`, not an error — being the wrong
shape is a fact about the document, and only profile violations are
errors.

## What's in the canonical profile we implement

- **Definite length only.** Indefinite-length items (`0x9F`, `0xBF`,
  `0x5F`, `0x7F`, `0xFF` break) are rejected.
- **Smallest-form integer encoding.** A 1-byte form whose payload
  fits inline, or a 2-byte form whose payload fits in 1 byte, etc.,
  is rejected.
- **Length-first map keys.** When encoding a map: sort entries by
  the length of each *encoded key*; ties broken by bytewise lex.
  Decoder verifies the order.
- **Duplicate map keys rejected.**
- **Bounded processing.** Decode and encode both cap nesting at 128 levels;
  one encoded item is capped at 1 MiB.
- **No floats, no undefined.** Vault records do not need them; the
  decoder rejects both.
- **Tags pass through.** This crate does not interpret tag
  semantics — that's a higher layer.

## What's not (yet)

- Floats (half/single/double): future work; the shortest-form
  preserve-the-value rule is non-trivial.
- Streaming encode/decode: this crate is allocation-friendly and
  small-message-friendly; large blobs go through VLT14 attachments
  which has its own framing.

## Where it fits in the Vault stack

Sits beside other format primitives. Consumed by:

- **VLT02** (`vault-records`) — every record's plaintext bytes are
  canonical CBOR; that's what gives us "one record, one byte sequence"
  for AEAD AAD binding.
- **VLT05** (`vault-auth`, FIDO2-PRF / WebAuthn) — COSE-Key
  formatting.
- **VLT09** (audit log) — entries are canonical CBOR so the hash
  chain is reproducible.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`CBR01-canonical-cbor.md`](../../../specs/CBR01-canonical-cbor.md).
The language-neutral vectors live in
[`canonical-cbor-v1`](../../../specs/fixtures/canonical-cbor-v1/README.md) and
are consumed directly by the Rust, C, and C++ implementations.

## Citations

- RFC 8949 — *Concise Binary Object Representation (CBOR)*. The §4.2
  series defines the deterministic profile; this crate implements
  §4.2.3 (Length-First Map Key Ordering).
- RFC 9052 (COSE) and the FIDO2 CTAP 2.1 spec, both of which use
  this canonical profile and are downstream consumers.
