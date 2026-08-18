# CBR01 - Canonical CBOR Codec

## Overview

This contract defines a from-scratch codec for CBOR (RFC 8949) using the
deterministic profile in section 4.2.3, "Length-First Map Key Ordering." That is
the ordering used by COSE, CTAP2, and WebAuthn.

Canonical CBOR lives beside the repository's other pure format primitives. It
is consumed by Vault records, authentication material, password-manager formats,
and audit entries. The contract is language-neutral. Rust is the first
established implementation; C and C++ are independent emerging-lane reference
oracles. No implementation is normative by itself.

## Why this primitive exists

Vault layers require one logical value to have exactly one byte sequence. Plain
CBOR allows several encodings of the same integer, arbitrary map order, and
indefinite lengths. Those choices break byte-bound authentication, deterministic
revision comparison, COSE-Key interoperability, and reproducible audit hashes.

## Value model

| Neutral kind | Payload | CBOR major/simple type |
|---|---|---|
| `unsigned` | unsigned 64-bit integer | major 0 |
| `negative` | unsigned 64-bit `n`, representing `-1 - n` | major 1 |
| `bytes` | byte sequence | major 2 |
| `text` | valid UTF-8 scalar sequence | major 3 |
| `array` | ordered values | major 4 |
| `map` | key/value pairs before canonicalization | major 5 |
| `tag` | unsigned 64-bit tag plus one value | major 6 |
| `bool` | false or true | simple 20 or 21 |
| `null` | no payload | simple 22 |

Floats and `undefined` are outside v1. Floats require their own shortest-form
preserve-the-value rule; deterministic callers use `null` for absence.

## Canonical profile

The encoder produces and the decoder enforces all of these rules:

| Rule | Encoder | Decoder |
|---|---|---|
| Definite lengths only | always definite | rejects additional info 31 |
| Smallest-form arguments | uses inline, 1, 2, 4, or 8 bytes minimally | rejects expanded integers, lengths, and tags |
| Length-first map keys | sorts encoded keys by length, then unsigned bytewise lexicographic order | requires keys to be strictly increasing under the same order |
| Unique map keys | rejects identical encoded keys | rejects duplicates as non-canonical order |
| Reserved additional info | never emits 28, 29, or 30 | rejects 28, 29, and 30 |
| UTF-8 text | accepts language strings | validates all text bytes |
| Closed simple values | emits false, true, and null | rejects every other simple value |
| No floats | never emits floats | rejects half, single, and double floats |
| Exactly one item | emits one item | rejects trailing bytes |
| Payload-blind errors | returns only stable identifiers and literal diagnostics | never reflects input bytes in an error |

## Language-neutral operations

Each implementation exposes the following behavior with language-native names
and result types:

- `encode_checked(value) -> bytes | error` validates the whole value before it
  publishes any bytes.
- `decode(bytes) -> value | error` consumes exactly one canonical item.
- An append-oriented checked encoder, when offered, leaves the destination
  unchanged on failure.
- A legacy infallible convenience wrapper may remain for source compatibility,
  but it must delegate to the checked encoder and must never emit bytes for an
  invalid value.
- A span-reporting map decoder, when offered, accepts exactly what `decode`
  accepts and additionally reports, for each entry, where that entry's value
  sat in the input. See *Value spans* below.

## Value spans

An implementation may offer a decoder that reports the byte range each map
entry's value occupied in the input. It is optional: it changes no wire
behavior, adds no error identifier to the table below, and an implementation
without it is fully conformant.

What makes such a decoder well-defined is the profile itself. Every rule the
encoder applies is also enforced by the decoder — smallest-form arguments,
definite lengths only, length-first key order with no duplicates — so an input
that decodes has exactly one legal spelling, and that spelling is the one the
encoder emits. For any value `v` decoded from a span `s` of input `b`:

```text
    b[s] == encode_checked(v)      whenever encode_checked(v) succeeds
```

The qualifier matters and is the reason the operation is worth offering. The
decoder has no input-length bound; `max_encoded_bytes` bounds the *encoder*
only. Inputs therefore exist that decode and will not re-encode, and for those
`b[s]` is the value's canonical bytes even though `encode_checked` reports
`encode-too-large`. A caller that passes an unrecognised sub-document through —
holding it now, re-emitting it later without interpreting it — must take the
span rather than re-encode the value, because the span cannot fail on any input
that decoded at all.

Two obligations on an implementation that offers it:

- The spans must come from the same parse that validates the input, not from a
  second, more permissive scanner. A span-reporting decode must reject exactly
  what `decode` rejects, with the same identifier.
- Input that is valid canonical CBOR but is not a map is a shape mismatch to be
  reported in the host language's own vocabulary (an absent result, a distinct
  status), not a profile violation. Only profile violations are errors.

## Portable resource limits

The v1 profile fixes two limits:

- `max_nesting_depth = 128`. The root is depth zero. Exactly 128 nested arrays
  or tags are accepted. A value requiring depth 129 is `encode-too-deep` while
  encoding or `too-deep` while decoding.
- `max_encoded_bytes = 1_048_576`. This includes the complete item, headers
  included. A checked encoder rejects a larger result before publishing bytes
  with `encode-too-large`.

Higher layers may impose smaller limits. They may not silently increase these
portable-oracle limits.

## Stable error identifiers

| Error ID | Meaning |
|---|---|
| `unexpected-eof` | input ended within an item |
| `trailing-bytes` | bytes remain after one item |
| `reserved` | additional info 28, 29, or 30 |
| `indefinite` | an indefinite item or break marker |
| `non-minimal-integer` | integer, length, or tag used a longer form |
| `invalid-utf8` | a text payload is not UTF-8 |
| `non-canonical-map-order` | decoded keys are not strictly length-first |
| `unsupported-simple` | simple value is not false, true, or null |
| `float-not-supported` | half, single, or double float |
| `too-deep` | decoded nesting exceeds 128 |
| `length-too-large` | a declared input length/count cannot fit or exceeds remaining bytes |
| `duplicate-map-key` | two input keys have identical canonical encodings |
| `encode-too-deep` | value nesting exceeds 128 during encoding |
| `encode-too-large` | encoded item would exceed 1,048,576 bytes |

Allocation failure is a language/runtime host error, not a wire-conformance
identifier. Implementations with explicit allocation results may expose it as a
separate status.

Every human-readable conformance diagnostic begins with the literal prefix
`canonical-cbor:` and contains no payload-derived value, key material, numeric
length, offset, or input byte.

## Map ordering in detail

Given a map with entries `(key, value)`:

1. Compute the checked canonical encoding of every key.
2. Sort by encoded-key length ascending, then unsigned bytewise lexicographic
   order.
3. If adjacent encoded keys are equal, fail with `duplicate-map-key` and emit no
   map bytes.
4. Emit the definite map header followed by each encoded key and checked value
   in the strict order.

The decoder compares the exact on-wire key spans. Every consecutive key must be
strictly greater under the same ordering, which rejects both duplicates and
out-of-order keys.

Section 4.2.1's bytewise-only order is preferred for some new protocols, but
COSE-Key and CTAP2 require section 4.2.3. A future profile may add bytewise-only
ordering explicitly; v1 never guesses between them.

## Threat model and validation

| Threat | Defense |
|---|---|
| Alternative encodings create hash/authentication collisions | strict shortest-form decoder and deterministic encoder |
| Duplicate logical keys create ambiguous objects | checked encoder rejects duplicate encoded keys; decoder requires strict order |
| Invalid UTF-8 reaches higher layers | decoder validates scalar sequences |
| Deep nesting exhausts the stack | encode and decode depth caps at 128 |
| Huge output exhausts memory | checked encoder caps the complete item at 1 MiB |
| Huge declared input length triggers allocation | decoder checks platform width and remaining bytes before allocation |
| Malicious bytes enter logs | static payload-blind error identifiers and diagnostics |
| A valid prefix hides trailing data | decode consumes exactly one item |

## Shared conformance fixture

`code/specs/fixtures/canonical-cbor-v1/cases.json` is the normative executable
corpus. Its schema pins the profile, both limits, exact accepted hex encodings,
stable rejection identifiers, and encode-only hostile-value builders.

`canonical_cbor_vectors.h` is a generated dependency-free projection consumed
by the C, C++, and Rust reference tests. The repository fixture test regenerates
it in memory and requires byte-for-byte equality, so the projection cannot drift
from the JSON source.

The corpus covers:

- all integer and length header boundaries;
- bytes, UTF-8 text, arrays, maps, tags, booleans, and null;
- heterogeneous length-first key ordering and input-order independence;
- duplicate keys, non-minimal arguments, indefinite forms, reserved values,
  floats, unsupported simple values, invalid UTF-8, truncation, trailing bytes,
  hostile lengths, depth 128/129, and the encode-size limit;
- static error mapping with no input reflection.

## Non-goals

- floats;
- streaming encode/decode;
- CBOR diagnostic notation;
- tag semantics;
- cryptographic framing, schema evolution, signatures, storage, or I/O.

## Citations

- RFC 8949, *Concise Binary Object Representation (CBOR)*, especially sections
  4.2 and 4.2.3.
- RFC 9052 (COSE), section 1.4.
- FIDO2 CTAP 2.1, section 6.
