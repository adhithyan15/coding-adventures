# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added

- Initial implementation of the canonical CBOR codec in Go (`canonicalcbor`),
  conforming to RFC 8949 §4.2.3 (Length-First Map Key Ordering) and CBR01.
- `Value` algebraic model with defensive copying constructors and accessors for
  all 9 neutral kinds (`unsigned`, `negative`, `bytes`, `text`, `array`, `map`,
  `tag`, `bool`, `null`).
- `EncodeChecked` and atomic `EncodeIntoChecked` encoders enforcing smallest-form
  integers, length-first map key ordering, duplicate map key rejection,
  `MaxNestingDepth` (128), and `MaxEncodedBytes` (1,048,576).
- `Decode` parser enforcing strict definite lengths, minimal integer encoding,
  UTF-8 validity, depth boundaries, length bounds checks prior to allocation,
  and trailing byte rejection.
- `DecodeMapSpans` reporting exact half-open byte spans for map entry values with
  guaranteed equivalence to `EncodeChecked(value)`.
- All 14 closed, payload-blind error IDs with static "canonical-cbor: " literal
  descriptions.
- Shared conformance vector execution against `code/specs/fixtures/canonical-cbor-v1/cases.json`.
- Boundary, property-based round-trip, permutation invariance, non-minimal header
  widening rejection, and native fuzz tests (`FuzzDecode`, `FuzzEncodeDecodeBijection`).
