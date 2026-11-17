# Changelog

All notable changes to the `cpp/hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `cpp/hamurabi`: header-only pure deterministic ISO C++20 implementation of the 1968 Hamurabi resource allocation simulation (GAME01).
- Value classes and structures under namespace `ca::hamurabi`: `HamurabiState`, `TurnOrder`, `GameStatus`, `Rating`, and `HamurabiError`.
- Core simulation APIs:
  - `new_game`: deterministic initialization with Knuth PCG32 LCG and Year 1 land price draw.
  - `validate_order`: canonical 7-step precedence validation enforcing order bounds and resource limits.
  - `step_year`: strictly ordered PRNG draw sequence for harvest yield, rat loss, plague, starvation, immigration, and land price.
  - `format_report`: authentic teletype annual status report.
  - `error_message`: string representation of error codes.
- Conformance and Security Verification:
  - `VER07` zero raw `new`/`delete` allocation.
  - `CWE-190` protection on `INT32_MIN` negation and `std::rotr` (`<bit>`) rotation.
  - Unit test suite (`tests/hamurabi_test.cpp`) and full 31-case conformance test suite (`tests/conformance_test.cpp`).
