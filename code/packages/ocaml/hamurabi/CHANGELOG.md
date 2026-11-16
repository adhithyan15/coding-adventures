# Changelog

All notable changes to the `coding-adventures-hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `coding-adventures-hamurabi`: pure deterministic simulation core for the seminal 1968 Hamurabi game (GAME01).
- Complete immutable record implementations for `hamurabi_state` and `turn_order`.
- Hardened 64-bit PCG32 PRNG with unsigned `Int64` bit shifts and `0xFFFFFFFFL` masking.
- Guard against `Int32.min_int` negation and overflow.
- Full 8-variant `hamurabi_error` result hierarchy.
- Authentic 1970s teletype report generator (`format_report`).
- Zero third-party dependency conformance test suite using `alcotest` parsing `code/specs/fixtures/hamurabi-v1/cases.txt`.
