# Changelog

All notable changes to the `coding_adventures_hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `coding_adventures_hamurabi`: deterministic simulation core for the seminal 1968 mainframe resource allocation game (GAME01).
- Core domain structures:
  - `HamurabiState`: 15-field pure record tracking year, population, bushels, acres, price, harvest yield, rat loss, starvation history, immigration, plague status, and PCG32 RNG state.
  - `TurnOrder`: player decisions per turn (`acresDelta`, `bushelsToFeed`, `acresToPlant`).
  - `GameStatus`: sealed hierarchy (`Playing`, `Won(Rating)`, `Impeached(starvedThisTurn, percent)`, `GameOverExtinction`).
  - `Rating`: historical performance evaluation (`charlemagne`, `acre`, `nero`, `disaster`).
  - `HamurabiError`: 8-variant enum for invalid orders and terminal state guards.
  - `Result<T, E>`: sealed algebraic type with `Ok` and `Err`.
- Core simulation APIs:
  - `newGame(BigInt seed)` / `newGameFromInt(int seed)`: initializes simulation state with canonical 1968 baseline parameters.
  - `validateOrder(HamurabiState state, TurnOrder order)`: pure validation enforcing order bounds, resource availability, and terminal state checks.
  - `stepYear(HamurabiState state, TurnOrder order)`: executes one simulation year with exact PCG32 draw sequence.
  - `formatReport(HamurabiState state)`: authentic 1970s mainframe teletype annual status report.
- Verification and Assurance:
  - Conformance test suite (`test/conformance_test.dart`) executing against `code/specs/fixtures/hamurabi-v1/cases.json`.
  - Security and unit test suite (`test/hamurabi_test.dart`) covering all 8 error arms, CWE-190/CWE-681 mitigations using `BigInt` for 64-bit unsigned PCG32 state and bounds checks on domain types.
  - Capability declaration in `required_capabilities.json` with `"capabilities": []`.
