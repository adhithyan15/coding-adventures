# Changelog

All notable changes to the C `canonical-cbor` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `canonical-cbor` crate — a deterministic
  CBOR (RFC 8949) codec in the "length-first map key ordering" canonical profile.
- A heap-owned recursive `CborValue` tree with constructors (`cbor_unsigned`,
  `cbor_negative`, `cbor_bool`, `cbor_null`, `cbor_bytes`, `cbor_text`,
  `cbor_array`/`cbor_array_push`, `cbor_map`/`cbor_map_push`, `cbor_tag`),
  `cbor_free`, and deep `cbor_equal`.
- `cbor_encode` (canonical bytes into a malloc'd buffer, smallest-form integers,
  stable length-first map key ordering) and a strict `cbor_decode` that rejects
  every non-canonical input: expanded integers, indefinite lengths, reserved
  info, non-UTF-8 text, non-canonical/duplicate map keys, unsupported simples,
  floats, over-deep nesting, and over-large declared lengths.
- `CborStatus` + out-parameter API in place of the Rust `Vec` / `Result`; a
  from-scratch UTF-8 validator (same acceptance set as `std::str::from_utf8`).
- Security-hardened decoder: recursion-depth cap (`CBOR_MAX_DECODE_DEPTH`),
  declared lengths bounded by remaining input before any allocation, and
  overflow-checked cursor and container-growth arithmetic.
- 200 checks mirroring the Rust crate's own unit tests (byte-exact encode
  vectors, every decoder error path, round-trips, canonical map ordering, and
  DoS-defence cases), run under every available C compiler via the shared
  `iso-harness`; the suite also passes clean under AddressSanitizer +
  UndefinedBehaviorSanitizer (exercising the ownership/error-cleanup paths).
