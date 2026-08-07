# Changelog

All notable changes to the C++ `canonical-cbor` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `canonical-cbor` crate
  (namespace `ca::canonical_cbor`) — a deterministic CBOR (RFC 8949) codec in
  the "length-first map key ordering" canonical profile.
- A fully value-semantic `CborValue` (copyable, `==`-comparable) built from
  `std::vector` / `std::string` — no pointers, no manual memory — with factories
  `unsigned_val`, `negative`, `boolean_val`, `null`, `byte_string`,
  `text_string`, `arr`, `mapping`, `tag`.
- `encode` returning `std::vector<uint8_t>` (smallest-form integers, stable
  length-first map key ordering via `std::stable_sort`), and a strict `decode`
  that throws `CborException(CborError)` on every non-canonical input; a
  from-scratch UTF-8 validator matching `std::str::from_utf8`.
- Security-hardened decoder: recursion-depth cap (`MAX_DECODE_DEPTH`), declared
  lengths bounded by remaining input before allocation, overflow-checked cursor
  arithmetic.
- 179 checks mirroring the Rust crate's own unit tests, run under every
  available C++ compiler via the shared `iso-harness`; the suite also passes
  clean under AddressSanitizer + UndefinedBehaviorSanitizer.
