# Changelog

All notable changes to the `Hamurabi` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `Hamurabi`: deterministic simulation core for the seminal 1968 mainframe resource allocation game (GAME01).
- Core domain structures:
  - `HamurabiState`: 15-field pure value struct tracking year, population, bushels, acres, price, harvest yield, rat loss, starvation history, immigration, plague status, and PCG32 RNG state.
  - `TurnOrder`: player decisions per turn (`acresDelta`, `bushelsToFeed`, `acresToPlant`).
  - `GameStatus`: exhaustive enum (`playing`, `won(rating:)`, `impeached(starvedThisTurn:percent:)`, `gameOverExtinction`).
  - `Rating`: historical performance evaluation (`charlemagne`, `acre`, `nero`, `disaster`).
  - `HamurabiError`: 8-variant enum for invalid orders and terminal state guards.
  - Full `Codable`, `Equatable`, `Hashable`, and `Sendable` conformance.
- Core public simulation APIs:
  - `newGame(seed: UInt64) -> HamurabiState`: initializes state with canonical 1968 baseline parameters.
  - `validateOrder(state:order:) -> Result<Void, HamurabiError>`: pure validation enforcing order bounds, resource availability, and terminal state checks.
  - `stepYear(state:order:) -> Result<HamurabiState, HamurabiError>`: executes one simulation year with exact PCG32 draw sequence.
  - `formatReport(state:) -> String`: authentic 1970s mainframe teletype annual status report.
- Verification and Assurance:
  - Conformance test suite (`Tests/HamurabiTests/ConformanceTests.swift`) executing against `code/specs/fixtures/hamurabi-v1/cases.json` using `Foundation.JSONDecoder`.
  - Security and unit test suite (`Tests/HamurabiTests/HamurabiTests.swift`) covering all 8 error arms, CWE-190 overflow guards, Int32.min negation safety, and adversarial security fixtures.
  - Capability declaration in `required_capabilities.json` with `"capabilities": []`.
