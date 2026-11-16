# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-14

### Added
- Initial pure-Elixir implementation of RFC 8949 deterministic canonical CBOR (§4.2.3 Length-First Map Key Ordering / CBR01 profile).
- Distinct algebraic value model in `CodingAdventures.CanonicalCbor.Types`:
  `CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMap`,
  `CborTag`, `CborBool`, and `CborNull`.
- Pair model `CborMapEntry` and half-open wire span models `Span` and `SpannedMapEntry`.
- Symmetrical `MAX_NESTING_DEPTH = 128` nesting limits across arrays, tags, map values, and map keys.
- Centralized protocol constants in `Constants` with zero inline magic literals.
- Consolidated payload-blind diagnostic strings and `<redacted>` security placeholder in `Messages` (Standing Rule #20).
- Pure computation with `capabilities: []` and zero runtime Hex dependencies (`deps: []`).
- 55/55 test vectors passing against `cases.json`, 10,000-iteration adversarial mutation fuzz loop, deterministic PRNG property tests, and 7-class BEAM/Elixir CVE resilience showcase with >=95% test coverage.
