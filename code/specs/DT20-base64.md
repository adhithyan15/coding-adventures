# DT20 — Base64 (RFC 4648)

## Why this exists

Twelve files in `code/packages/rust/` carry their own copy of the base64
alphabet, and three more (`smart-home-onvif-integration`,
`smart-home-axis-vapix-integration`, `smart-home-camera-media-http-executor`)
depend on the external `base64` crate. None of them agree on which variants
they support, and none of them can be reused, because each is a private helper
inside a crate that needed to encode one thing.

Base64 is a forty-line algorithm with a normative test vector table. Having
twelve of it is not a design; it is what happens when nobody has somewhere to
put it.

There is also a concrete consumer waiting. `MediaAssetRecord.data` is a
`Vec<u8>` that serialises as a JSON array of decimal numbers — 4.6 bytes on the
wire per media byte, and the reason `engram-anki-package` carries a 32 MiB
media ceiling on `wasm32`. Base64 is 1.33 bytes per byte. Making that change
needs a base64 implementation this repository owns.

## Scope

Encoding and decoding for the two alphabets RFC 4648 defines, each with and
without padding:

| Variant | Alphabet | Padding | RFC 4648 |
|---|---|---|---|
| `STANDARD` | `A–Z a–z 0–9 + /` | `=` | §4 |
| `STANDARD_NO_PAD` | `A–Z a–z 0–9 + /` | none | §3.2 |
| `URL_SAFE` | `A–Z a–z 0–9 - _` | `=` | §5 |
| `URL_SAFE_NO_PAD` | `A–Z a–z 0–9 - _` | none | §5, §3.2 |

Out of scope: base32, base16, MIME line wrapping (RFC 2045 §6.8), and
alternative alphabets. They can be added when something needs them; guessing
now would be inventing requirements.

## Interface

```rust
pub struct Alphabet { /* 64 encode symbols, a 256-entry decode table, padding */ }

pub const STANDARD: Alphabet;
pub const STANDARD_NO_PAD: Alphabet;
pub const URL_SAFE: Alphabet;
pub const URL_SAFE_NO_PAD: Alphabet;

pub fn encode(input: &[u8], alphabet: &Alphabet) -> String;
pub fn encode_into(input: &[u8], alphabet: &Alphabet, out: &mut String);
pub fn decode(input: &str, alphabet: &Alphabet) -> Result<Vec<u8>, DecodeError>;

pub fn encoded_len(input_len: usize, alphabet: &Alphabet) -> usize;
pub fn decoded_len_estimate(input_len: usize) -> usize;
```

`encode_into` exists because the motivating consumer is a streaming JSON writer
that already owns an output buffer. An API that can only return a fresh
`String` forces an allocation and a copy per media asset, which is the class of
amplification this work is trying to remove.

## Decoding is strict

A decoder that accepts malformed input silently produces bytes that were never
encoded. Each of these is an error, not a repair:

- **A character outside the alphabet**, including whitespace and newlines.
  Callers that need to strip whitespace should do so deliberately.
- **The wrong alphabet's characters** — `-`/`_` under `STANDARD`, `+`/`/`
  under `URL_SAFE`. Silently accepting both is how a URL-safe payload gets
  decoded by a standard decoder and appears to work until a byte lands on a
  `+`.
- **Padding on a `NO_PAD` alphabet**, and missing or over-long padding on a
  padded one.
- **A length of 1 modulo 4**, which no input can produce.
- **Non-zero discarded bits** in the final quantum. `"ZG=="` decodes `d` from
  six bits and discards two — but those two are set, so the encoder that
  produced it was not encoding `d`. Accepting it means two distinct strings
  decode to the same bytes, and a signature or hash computed over the decoded
  value stops being a function of the input.

`DecodeError` names the byte offset and what was wrong there.

## Verification

**RFC 4648 §10 test vectors are the oracle.** The ten canonical
`""`/`"f"`/`"fo"`/`"foo"`/`"foob"`/`"fooba"`/`"foobar"` cases pin the padding
behaviour at every length modulo 3, which is where hand-written base64 goes
wrong. They are asserted verbatim, not derived from the implementation.

**Round-trip over every length 0..=1024**, so no length class is untested —
the tail cases are the ones that differ between correct and nearly-correct
implementations.

**Every rejection above has a test**, because a strict decoder that is strict
only in the docstring is the failure this section exists to prevent.

**Cross-checked against an independent implementation.** Python's
`base64.b64encode` / `urlsafe_b64encode` are in the standard library and were
not written from this spec, which is the property that makes them useful here.

## Non-goals

- Constant-time decoding. Base64 is not a secret-handling primitive, and
  claiming timing resistance the implementation does not have would be worse
  than not claiming it.
- SIMD. The consumer encodes media at import time, not in a loop that matters.
