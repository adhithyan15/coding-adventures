# Changelog

All notable changes to the `CodingAdventures.Hamurabi.CSharp` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial C# release of `hamurabi`: deterministic simulation core for the seminal 1968 mainframe resource allocation game (GAME01).
- Pure immutable value types:
  - `HamurabiState`: 15-field record tracking year, population, bushels, acres, price, harvest yield, rat loss, starvation history, immigration, plague status, and RNG state.
  - `TurnOrder`: player decisions per turn (`AcresDelta`, `BushelsToFeed`, `AcresToPlant`).
  - `GameStatus`: `Playing`, `Won(Rating)`, `Impeached(StarvedThisTurn, Percent)`, `GameOverExtinction`.
  - `Rating`: historical performance evaluation (`Charlemagne`, `Acre`, `Nero`, `Disaster`).
  - `HamurabiError`: exhaustive 8-variant enum for invalid orders and terminal state guards.
  - `Result<T, E>` and `Unit`: lightweight result types for pure functional error reporting without exceptions.
- Core public simulation APIs in `Hamurabi`:
  - `NewGame(ulong seed)` / `new_game`: initializes canonical 1968 baseline parameters (100 people, 2800 bushels, 1000 acres, initial land price draw from [17, 26]).
  - `ValidateOrder(HamurabiState, TurnOrder)` / `validate_order`: validates order bounds, resource availability, and state invariants.
  - `StepYear(HamurabiState, TurnOrder)` / `step_year`: executes one simulation year with exact PCG32 draw sequence.
  - `RateReign(uint avgStarved, uint acresPerPerson)` / `rate_reign`: evaluates ruler performance.
  - `FormatReport(HamurabiState)` / `ReportYear` / `report_year`: authentic 1970s mainframe teletype report.
- Hardened arithmetic and CVE protections:
  - `unchecked` `ulong` wrapping math for PCG32 LCG and rotation (`BitOperations.RotateRight`).
  - `checked` 64-bit arithmetic and explicit guards against CWE-190/191 integer overflow (including `int.MinValue` on `AcresDelta`, revenue overflow, and harvest overflow).
  - Pure records without reflection serialization or `BinaryFormatter` (CWE-502).
  - Closed capability declaration in `required_capabilities.json` (`capabilities: []`).
