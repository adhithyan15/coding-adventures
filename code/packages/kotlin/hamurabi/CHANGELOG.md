# Changelog

All notable changes to the `kotlin/hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of Kotlin/JVM `hamurabi` package.
- Pure immutable data classes: `HamurabiState`, `TurnOrder`, `GameStatus`, `Rating`, `HamurabiError`, and `Result`.
- Core simulation engine functions:
  - `newGame(seed)`: PCG32 deterministic setup with stream 54 (`rngInc = 109`) and Year 1 land price draw `[17, 26]`.
  - `validateOrder(state, order)`: 8-step canonical constraint checking with integer overflow guards.
  - `stepYear(state, order)`: two-phase state transition (validation followed by 6-step PRNG draw sequence).
  - `formatReport(state)`: authentic 1970s mainframe teletype report formatter.
- Conformance test suite parsing `code/specs/fixtures/hamurabi-v1/cases.txt` across all 31 test cases.
- Zero-capability declaration in `required_capabilities.json`.
