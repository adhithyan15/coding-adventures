# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added `decode_map_spanned` and `SpannedMapEntry`: decode a canonical CBOR map
  and learn where each entry's value sat in the input. The bytes a span points
  at are exactly what re-encoding the value would produce, because the strict
  decoder enforces every rule the encoder applies, so an input that decodes has
  only one legal spelling.

  It exists for callers that pass an unrecognised sub-document through — hold it
  now, re-emit it later, without interpreting it. Such a caller must slice
  rather than re-encode, because `decode` has no input-length bound while
  `try_encode` has `MAX_ENCODED_SIZE`: inputs exist that decode and will not
  re-encode, and for those a round trip turns a readable document into an error.
  Slicing a range the parser itself measured cannot fail on any input that
  decoded. `vault-records`' opaque-record arm is the first caller, where the
  round trip was denying vault open outright.

  Spans come from the same parse that validates the input — the map reader is
  now one function used by both decoders — so the spanned decode rejects
  exactly what `decode` rejects, with the same error, pinned by a differential
  test over sixteen hostile inputs. Valid CBOR that is not a map is `Ok(None)`
  rather than an error, so no error identifier is added to the portable CBR01
  set and the C and C++ ports need no change.
- Added `try_encode` and transactional `try_encode_into` APIs with stable,
  payload-blind errors for duplicate encoded map keys, excessive nesting, and
  output beyond the 1 MiB item limit.
- Added direct execution of all language-neutral `canonical-cbor-v1` vectors,
  including exact boundary encodings and every closed rejection class.

### Changed

- `encode` and `encode_into` now delegate to the same checked encoder, so the
  legacy convenience API can no longer emit duplicate-key or unbounded items.

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of canonical (deterministic) CBOR
  encoding/decoding per RFC 8949 §4.2.3 ("Length-First Map Key
  Ordering" — the CTAP2 / COSE / WebAuthn profile).
- `CborValue` enum: `Unsigned`, `Negative`, `Bytes`, `Text`, `Array`,
  `Map`, `Tag`, `Bool`, `Null`. (Floats and `undefined` deliberately
  unsupported in v1.)
- `encode(&CborValue) -> Vec<u8>` produces deterministic bytes; the
  same input value always yields the same output. Map keys are
  reordered length-first then bytewise lex at encode time, so callers
  may pass entries in any order.
- `decode(&[u8]) -> Result<CborValue, CborError>` is strict canonical:
  rejects non-minimal integer/length encodings, indefinite-length
  items, reserved additional-info values, non-canonical map order,
  duplicate map keys, invalid UTF-8 in text strings, floats,
  `undefined`, and unassigned simple values. Trailing bytes after
  one decoded item are an error.
- `CborError` typed enum with `Display` strings sourced only from
  literals — never from the input bytes — so a malicious payload
  cannot inject error-message content.
- 50 unit tests covering: smallest-form integer encoding (inline,
  1-byte, 2-byte, 4-byte, 8-byte boundaries), decoder rejection of
  non-minimal forms, negative integers (-1, -24, -25 boundary),
  bytes/text encoding and UTF-8 validation, array order preservation,
  length-first map ordering with same-length tiebreak, map-canonical-
  order rejection on decode, duplicate-key rejection, complex
  nested-structure round-trip with canonical-order idempotence,
  large-array (1000 elements) round-trip, large-map (100 entries)
  shuffled-input determinism, tag round-trips, indefinite-length
  rejection, reserved-info rejection, undefined/float rejection,
  trailing-bytes rejection, EOF in header / argument / byte-string,
  recursion-depth caps (deeply-nested arrays and tags rejected;
  nesting at the limit accepted), array / map / byte-string
  oversized-length rejection without pre-allocation, max-u64-length
  rejection, and that error `Display` strings always start with the
  static prefix `"canonical-cbor:"`.

### Security review

Round 1 review found 1 CRITICAL + 2 HIGH. All fixed before push:

- **CRITICAL (Finding 1)** — unbounded recursion in `read_value`
  could be exploited by a small input chain (`0x81 0x81 0x81 …` or
  `0xC6 0xC6 0xC6 …`) to blow the OS stack. Fixed: introduced
  `MAX_DECODE_DEPTH = 128` cap; depth is threaded through every
  recursive call (array element, map key, map value, tag inner);
  excess depth returns `CborError::TooDeep`.
- **HIGH (Finding 2)** — `Vec::with_capacity(arg)` on attacker-
  controlled length could pre-allocate gigabytes from a 9-byte
  payload. Fixed: introduced `length_within_remaining(arg,
  remaining, min_per_unit)` that rejects any declared length whose
  minimum wire-byte cost exceeds the remaining input; bytes/text
  use `min_per_unit = 1`, arrays `1`, maps `2`. Excess returns
  `CborError::LengthTooLarge`.
- **HIGH (Finding 3)** — `usize` overflow in `Cursor::read_n`'s
  `pos + n` arithmetic plus silent `u64 → usize` truncation could
  bypass the bounds check on 32-bit targets. Fixed: `read_n` uses
  `checked_add`; the u64-to-usize conversion uses `try_from`.

Round 2 review confirmed all three fixes correct and introduced
no new vulnerabilities — clean pass.
