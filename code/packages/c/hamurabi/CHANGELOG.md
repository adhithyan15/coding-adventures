# Changelog

All notable changes to the `c/hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `c/hamurabi`: pure deterministic ISO C17 implementation of the 1968 Hamurabi resource allocation simulation (GAME01).
- Core types and structures: `hamurabi_state`, `hamurabi_turn_order`, `hamurabi_game_status`, `hamurabi_rating`, and `hamurabi_error`.
- Core simulation APIs:
  - `hamurabi_new_game`: deterministic initialization with Knuth PCG32 LCG and Year 1 land price draw.
  - `hamurabi_validate_order`: canonical 7-step precedence validation enforcing order bounds and resource limits.
  - `hamurabi_step_year`: strictly ordered PRNG draw sequence for harvest yield, rat loss, plague, starvation, immigration, and land price.
  - `hamurabi_format_report`: authentic teletype annual status report formatted into a caller-supplied buffer.
  - `hamurabi_error_string`: human-readable descriptions of error enum codes.
- Conformance and Security Verification:
  - `VER07` zero dynamic memory allocation in simulation core and caller-buffered report formatting.
  - `CWE-476` NULL pointer verification across all public entry points.
  - `CWE-190` signed overflow protection on `INT32_MIN` negation and bit-shift UB guards (`rot & 31u`).
  - Unit test suite (`tests/hamurabi_test.c`) and full 31-case conformance test suite (`tests/conformance_test.c`).
