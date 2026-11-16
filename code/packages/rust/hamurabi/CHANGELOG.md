# Changelog

All notable changes to the `hamurabi` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 — 2026-11-15

### Added
- Initial release of `hamurabi`: deterministic simulation core for the seminal 1968 mainframe resource allocation game (GAME01).
- Pure Knuth-style literate implementation in `src/lib.rs` under `#![forbid(unsafe_code)]`.
- Core data structures:
  - `HamurabiState`: 15-field pure record tracking year, population, bushels, acres, price, harvest yield, rat loss, starvation history, immigration, plague status, and RNG state.
  - `TurnOrder`: player decisions per turn (`acres_delta`, `bushels_to_feed`, `acres_to_plant`).
  - `GameStatus`: `Playing`, `Won(Rating)`, `Impeached { starved_this_turn, percent }`, `GameOverExtinction`.
  - `Rating`: historical performance evaluation (`Charlemagne`, `Acre`, `Nero`, `Disaster`).
  - `HamurabiError`: exhaustive 8-variant error enum for invalid orders and terminal state guards.
- Core public simulation APIs:
  - `new_game(seed: u64) -> HamurabiState`: initializes state with canonical 1968 baseline parameters (100 people, 2800 bushels, 1000 acres, initial land price draw from range [17, 26]).
  - `validate_order(state: &HamurabiState, order: TurnOrder) -> Result<TurnOrder, HamurabiError>`: pure validation enforcing order bounds, resource availability, and terminal state checks.
  - `step_year(state: &HamurabiState, order: TurnOrder) -> Result<HamurabiState, HamurabiError>`: executes one simulation year with exact PCG32 draw sequence:
    1. Harvest yield roll: `[1, 5]` bushels/acre.
    2. Rat infestation roll: `[1, 100]`; if `<= 40`, rat consumption factor roll: `[1, 5]`.
    3. Plague strike roll: `[1, 100]`; strikes if `<= 15`.
    4. Starvation evaluation: 20 bushels/person; impeachment check if starved > 45%.
    5. Plague mortality: halves surviving population if active.
    6. Immigration factor roll: `[1, 5]`; immigrants arrive only if zero starved.
    7. Next year land price roll: `[17, 26]`.
  - `format_report(state: &HamurabiState) -> String`: authentic 1970s mainframe teletype annual status report.
- Verification and Assurance:
  - `VER07` zero-heap-allocation microbenchmarks (`tests/microbenchmarks.rs`) with custom `CountingAllocator` and sabotage detection.
  - `VER08b` runtime capability confinement tests (`tests/runtime_capability.rs`).
  - Full conformance and regression suite (`tests/conformance.rs`) covering all 8 error arms, CWE-190 overflow guards, 10-year deterministic transcript, plague, impeachment, ratings, and adversarial security fixtures.
  - Hardened `validate_order` against CWE-400 / CWE-835 (enforcing odd `rng_inc` to prevent PCG32 period collapse and infinite loop, and bounds on `year`, `land_price`, and `harvest_per_acre`) and CWE-190 harvest overflow.
  - Hardened `step_year` against CWE-190 with saturating additions and bounded conversions.
  - Closed capability declaration in `required_capabilities.json` (`capabilities: []`) verified by `ca-capability-analyzer`.
  - Effect certificate (`effect_certificate.json`) proved zero side effects.
  - Assurance report (`assurance_report.json`) addressing all 26 CWEs per `VER15`.
