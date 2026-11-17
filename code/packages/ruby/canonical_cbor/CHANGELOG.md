# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added

- Initial implementation of the canonical CBOR codec in Ruby (`coding_adventures_canonical_cbor`),
  conforming to RFC 8949 §4.2.3 (Length-First Map Key Ordering) and CBR01.
- Immutable `CborValue` algebraic model with frozen instances and accessors for
  all 9 neutral kinds (`CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMap`,
  `CborTag`, `CborBool`, `CborNull`).
- Exact 64-bit unsigned integer handling with strict `[0, MAX_UINT64]` validation for `CborUnsigned`,
  `CborNegative`, and `CborTag` preventing Bignum wraparound.
- `encode_checked` and atomic `encode_into_checked` encoders enforcing smallest-form
  integers, length-first map key ordering, duplicate map key rejection,
  `MAX_NESTING_DEPTH` (128), and `MAX_ENCODED_BYTES` (1,048,576).
- `decode` parser enforcing strict definite lengths, minimal integer encoding,
  UTF-8 validity, depth boundaries, length bounds checks prior to collection allocation,
  and trailing byte rejection.
- Strict UTF-8 validation rejecting non-UTF-8 encodings (like `ASCII-8BIT`) and lone surrogates/CESU-8
  (CVE-5 encoding confusion defense).
- `decode_map_spans` reporting exact half-open byte spans for map entry values with
  guaranteed equivalence to `encode_checked(value)`.
- All 14 closed, payload-blind error IDs in `Messages` with static `canonical-cbor: ` literal descriptions.
- Shared conformance vector execution against `code/specs/fixtures/canonical-cbor-v1/cases.json` (55/55 passing).
- Comprehensive test suite achieving 100% line coverage and 100% branch coverage with SimpleCov,
  including integer boundary transitions, nesting depth and size transitions,
  7-class CVE Resilience Showcase, property round-trip bijection, map permutation invariance, and 10,000-iteration byte mutation fuzz loop.
