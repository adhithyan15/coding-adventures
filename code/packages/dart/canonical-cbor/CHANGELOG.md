# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added

- Initial implementation of the canonical CBOR codec in Dart (`coding_adventures_canonical_cbor`),
  conforming to RFC 8949 §4.2.3 (Length-First Map Key Ordering) and CBR01.
- Sealed `CborValue` algebraic model with defensive copying constructors and accessors for
  all 9 neutral kinds (`CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMap`,
  `CborTag`, `CborBool`, `CborNull`).
- Exact 64-bit unsigned integer handling using `BigInt` for `CborUnsigned`, `CborNegative`, and `CborTag`
  to overcome Dart VM signed 63-bit integer boundaries.
- `encodeChecked` and atomic `encodeIntoChecked` encoders enforcing smallest-form
  integers, length-first map key ordering, duplicate map key rejection,
  `maxNestingDepth` (128), and `maxEncodedBytes` (1,048,576).
- `decode` parser enforcing strict definite lengths, minimal integer encoding,
  UTF-8 validity, depth boundaries, length bounds checks prior to collection allocation,
  and trailing byte rejection.
- UTF-16 lone surrogate code-unit scanner (`validateUtf16ScalarString`) guarding against Dart's
  silent `U+FFFD` replacement behavior (CVE-5 collision resilience).
- `decodeMapSpans` reporting exact half-open byte spans for map entry values with
  guaranteed equivalence to `encodeChecked(value)`.
- All 14 closed, payload-blind error IDs with static `canonical-cbor: ` literal descriptions.
- Shared conformance vector execution against `code/specs/fixtures/canonical-cbor-v1/cases.json` (55/55 passing).
- Comprehensive test suite including integer boundary transitions, nesting depth and size transitions,
  7-class CVE Resilience Showcase, property round-trip bijection, map permutation invariance, and 10,000-iteration byte mutation fuzz loop.
