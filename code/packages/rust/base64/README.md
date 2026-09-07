# coding_adventures_base64

Base64 encoding and decoding per [RFC 4648](https://www.rfc-editor.org/rfc/rfc4648),
implemented from scratch with no dependencies.

## Why this exists

Twelve files under `code/packages/rust/` carried their own copy of the base64
alphabet, and three more depended on the external `base64` crate. None could be
reused, because each was a private helper inside a crate that needed to encode
one thing.

There is also a concrete consumer: `MediaAssetRecord.data` is a `Vec<u8>` that
serialises as a JSON array of decimal numbers — **4.6 bytes on the wire per
byte of media**, and the reason `engram-anki-package` carries a 32 MiB media
ceiling on `wasm32`. Base64 is **1.33**.

## Where it fits

A leaf: no dependencies, and nothing in the workspace is required to use it.
It is intended for the JSON facade (`engram-core-wasm`), the OAuth and
chief-of-staff crates that hand-rolled their own, and the smart-home crates
currently pulling `base64` from crates.io.

## Usage

```rust
use coding_adventures_base64::{decode, encode, encode_into, STANDARD, URL_SAFE_NO_PAD};

assert_eq!(encode(b"Man", &STANDARD), "TWFu");
assert_eq!(decode("TWFu", &STANDARD).unwrap(), b"Man");

// URL-safe swaps `+/` for `-_`; dropping padding avoids `%3D` escaping.
assert_eq!(encode(&[0xfb, 0xff], &URL_SAFE_NO_PAD), "-_8");

// Append into a buffer you already own -- the form a streaming writer wants.
let mut buffer = String::from("data:");
encode_into(b"foobar", &STANDARD, &mut buffer);
assert_eq!(buffer, "data:Zm9vYmFy");
```

### Variants

| Constant | Alphabet | Padding |
|---|---|---|
| `STANDARD` | `A–Z a–z 0–9 + /` | `=` |
| `STANDARD_NO_PAD` | `A–Z a–z 0–9 + /` | none |
| `URL_SAFE` | `A–Z a–z 0–9 - _` | `=` |
| `URL_SAFE_NO_PAD` | `A–Z a–z 0–9 - _` | none |

## Decoding is strict, deliberately

A decoder that repairs its input produces bytes nobody encoded. These are
errors, not repairs:

- a character outside the alphabet, **including whitespace and newlines**
- the *other* alphabet's characters — `-`/`_` under `STANDARD` and `+`/`/`
  under `URL_SAFE`, so a URL-safe payload cannot decode "correctly" until a
  byte happens to land on `+`
- padding on a `NO_PAD` variant, or the wrong amount on a padded one
- a length of 1 modulo 4, which no encoder can produce
- **non-zero discarded bits** in the final symbol: `"Zg=="` is `f`, and
  `"Zh=="` is not — accepting both would let two strings decode to one value,
  so a hash or signature over the decoded bytes stops being a function of the
  input

`DecodeError` names the byte offset and what was wrong there.

## Verification

- **RFC 4648 §10's own vectors**, transcribed verbatim rather than generated
  from this implementation.
- **Round-trip over every length 0..=1024** in all four variants — the tail
  cases are what separate correct from nearly-correct, and they recur every
  three bytes.
- **Cross-checked against Python's `base64` stdlib**: 300 lengths × 4 variants,
  zero mismatches. An independent implementation nobody wrote from this spec.
- Every documented rejection has a test.

## Non-goals

Base32, base16, MIME line wrapping, constant-time decoding (base64 is not a
secret-handling primitive), and SIMD.

See `code/specs/DT20-base64.md`.
