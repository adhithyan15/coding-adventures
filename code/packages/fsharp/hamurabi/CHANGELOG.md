# Changelog

All notable changes to the `CodingAdventures.Hamurabi.FSharp` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial F# release of `hamurabi`: deterministic simulation core for the seminal 1968 mainframe resource allocation game (GAME01).
- Pure immutable value types:
  - `HamurabiState`: 15-field record tracking year, population, bushels, acres, price, harvest yield, rat loss, starvation history, immigration, plague status, and RNG state.
  - `TurnOrder`: player decisions per turn (`AcresDelta`, `BushelsToFeed`, `AcresToPlant`).
  - `GameStatus`: `Playing`, `Won of Rating`, `Impeached of uint32 * uint32`, `GameOverExtinction`.
  - `Rating`: historical performance evaluation (`Charlemagne`, `Acre`, `Nero`, `Disaster`).
  - `HamurabiError`: exhaustive 8-variant discriminated union for invalid orders and terminal state guards.
  - Built-in F# `Result<'T, 'TError>` for pure functional error reporting without exceptions.
- Core public simulation APIs:
  - `newGame(seed: uint64)` / `new_game` / `NewGame`: initializes canonical 1968 baseline parameters (100 people, 2800 bushels, 1000 acres, initial land price draw from [17, 26]).
  - `validateOrder`: validates order bounds, resource availability, and state invariants.
  - `stepYear`: executes one simulation year with exact PCG32 draw sequence.
  - `rateReign`: evaluates ruler performance.
  - `formatReport` / `reportYear`: authentic 1970s mainframe teletype report.
- Hardened arithmetic and CVE protections:
  - `uint64` wrapping math for PCG32 LCG and rotation (`BitOperations.RotateRight`).
  - Explicit guards against CWE-190/191 integer overflow (including `Int32.MinValue` on `AcresDelta`, revenue overflow, and harvest overflow).
  - Pure immutable records without reflection serialization or `BinaryFormatter` (CWE-502).
  - Closed capability declaration in `required_capabilities.json` (`capabilities: []`).
