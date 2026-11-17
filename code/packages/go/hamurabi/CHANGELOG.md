# Changelog

All notable changes to the `hamurabi` Go package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial release of `hamurabi` in pure Go: deterministic simulation core for the seminal 1968 mainframe resource allocation game (`GAME01`).
- Domain data structures:
  - `HamurabiState`: 15-field immutable value record capturing full simulation state.
  - `TurnOrder`: player decisions per turn (`AcresDelta`, `BushelsToFeed`, `AcresToPlant`).
  - `GameStatus`: lifecycle status (`StatusPlaying`, `StatusWon`, `StatusImpeached`, `StatusGameOverExtinction`).
  - `Rating`: ruler classification (`RatingCharlemagne`, `RatingAcre`, `RatingNero`, `RatingDisaster`).
  - `HamurabiError`: closed enumeration of order refusal errors.
- Core public simulation APIs:
  - `NewGame(seed uint64) HamurabiState`: initializes state with canonical baseline parameters (100 citizens, 2800 bushels, 1000 acres, initial land price draw from [17, 26]).
  - `ValidateOrder(state HamurabiState, order TurnOrder) error`: pure validation enforcing order bounds, resource availability, and terminal state checks.
  - `StepYear(state HamurabiState, order TurnOrder) (HamurabiState, error)`: executes one simulation year with exact PCG32 draw sequence:
    1. Harvest yield roll: `[1, 5]` bushels/acre.
    2. Rat infestation roll: `[1, 100]`; if `<= 40`, rat consumption factor roll: `[1, 5]`.
    3. Plague strike roll: `[1, 100]`; strikes if `<= 15`.
    4. Starvation evaluation: 20 bushels/person; impeachment check if starved > 45%.
    5. Plague mortality: halves surviving population if active.
    6. Immigration factor roll: `[1, 5]`; immigrants arrive only if zero starved.
    7. Next year land price roll: `[17, 26]`.
  - `FormatReport(state HamurabiState) string`: authentic 1970s mainframe teletype annual status report.
- Verification and Assurance:
  - Zero heap allocation (`VER07`): `testing.AllocsPerRun == 0` for `StepYear` and `ValidateOrder`, plus benchmark suite (`BenchmarkStepYear`, `BenchmarkValidateOrder`).
  - Strict Go CVE/CWE hardening:
    - CWE-190 / CWE-191: signed/unsigned bounds checks, promotion of `-AcresDelta` via `int64` to guard against `math.MinInt32` overflow, saturating additions on `StarvedTotal` and `PercentStarvedSum`, and harvest overflow guard.
    - CWE-400 / CWE-835: rejection of even `RngInc` (`(RngInc & 1) != 1`) preventing PCG32 period collapse and infinite loops in rejection sampling.
  - Concurrency safety: concurrent goroutine race test proving zero shared mutable state.
  - Full shared conformance suite (`conformance_test.go`) validating all 31 cases in `cases.json`.
  - Zero ambient capability manifest in `required_capabilities.json` (`capabilities: []`).
