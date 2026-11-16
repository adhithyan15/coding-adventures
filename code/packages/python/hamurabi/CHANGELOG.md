# Changelog

All notable changes to the Python hamurabi package will be documented in this file.

## [0.1.0] - 2026-11-15

### Added

- Initial release of pure deterministic Hamurabi simulation core (`GAME01`).
- `HamurabiState` and `TurnOrder` immutable `@dataclass(frozen=True, slots=True)` value records.
- Deterministic PCG32 PRNG with `HAMURABI_STREAM_ID = 54` and explicit `(rng_state, rng_inc)` state tracking.
- Canonical validation sequence matching `GAME01` specification and Rust reference implementation.
- Authentic teletype report generator (`format_report`).
- Full cross-language conformance against `code/specs/fixtures/hamurabi-v1/cases.json`.
- Complete 26-CWE weakness closure tests including CWE-190, CWE-843, CWE-400, CWE-502, and CWE-94.
- Static capability audit verifying zero ambient capabilities (`CAP001`, `VER08a`).
