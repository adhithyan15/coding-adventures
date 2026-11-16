# Changelog

All notable changes to this package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial pure-Elixir implementation of `hamurabi` (GAME01): deterministic simulation core for the seminal 1968 mainframe resource allocation game.
- Pure value records:
  - `CodingAdventures.Hamurabi.State`: 15-field immutable struct capturing reign year, population, bushels, acres, market land price, harvest yield, rat consumption, starvation history, immigration, plague status, game status, and PCG32 state.
  - `CodingAdventures.Hamurabi.TurnOrder`: player order struct (`acres_delta`, `bushels_to_feed`, `acres_to_plant`) accepting structs and maps with atom or string keys.
- Core simulation engine API:
  - `new_game/1`: initializes state seeded with PCG32 (`stream_id = 54`, `inc = 109`) and Year 1 land price draw `[17, 26]`.
  - `validate_order/2`: validates player orders in strict canonical sequence (status check, invariant bounds, land trade affordability, food allocation, labor capacity, seed sufficiency, harvest bounds).
  - `step_year/2`: executes deterministic two-phase turn advancement with locked PCG32 draw sequence:
    1. Harvest yield draw `[1, 5]`.
    2. Rat infestation draw `[1, 100]` (if `<= 40`, factor draw `[1, 5]`).
    3. Plague strike draw `[1, 100]` (strikes if `<= 15`).
    4. Starvation and impeachment evaluation (`> 45%` starvation triggers impeachment).
    5. Plague mortality resolution (halves surviving population).
    6. Immigration factor draw `[1, 5]` (unconditional to maintain stream alignment).
    7. Next year land price draw `[17, 26]`.
    8. Turn advance and 10-year term evaluation (`Charlemagne`, `Acre`, `Nero`, `Disaster`).
  - `format_report/1`: authentic 1970s mainframe teletype annual status report.
  - Helper functions: `status_to_string/1`, `status_from_string/1`, `error_to_string/1`, `error_from_string/1`.
- Complete 26-CWE weakness closure and BEAM-specific hardening:
  - Strict `is_integer/1` guards across all inputs, rejecting floats, booleans, nil, strings, and atoms.
  - Explicit integer bounds enforcement (u8, u32, i32, u64).
  - Even `rng_inc` rejection to prevent infinite loop / short period collapse in PCG32 LCG.
  - Zero `String.to_atom/1` calls, eliminating BEAM atom table exhaustion (CWE-400).
  - Zero `Code.eval_*` or `:erlang.binary_to_term` calls, eliminating unsafe dynamic code evaluation (CWE-94 / CWE-502).
  - Declared `capabilities: []` with zero ambient authority (CAP001).
- 31/31 test vectors passing against `cases.json` fixture suite, full 10-year playthrough verification, and adversarial CWE test suite.
