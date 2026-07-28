# Changelog — mosfet-models

## Unreleased

### Added

- `Level1Params::pd` and `Level1Params::cjsw` add zero-default Berkeley drain
  diffusion perimeter and sidewall capacitance density, contributing
  `CJSW * PD` to the zero-bias drain-body capacitance.
- `Level1Params::as_` adds the zero-default Berkeley source diffusion area,
  contributing `CJ * AS` to the zero-bias source-body capacitance.
- `Level1Params::ad` and `Level1Params::cj` add zero-default Berkeley drain
  diffusion area and bottom-junction capacitance density, contributing
  `CJ * AD` to the zero-bias drain-body capacitance.
- `Level1Params::nrs` adds the Berkeley source diffusion square count,
  defaulting to one with finite, non-negative validation.
- `Level1Params::nrd` adds the Berkeley drain diffusion square count, defaulting
  to one with finite, non-negative validation.
- `Level1Params::rsh` adds zero-default Berkeley drain/source sheet resistance
  with finite, non-negative validation.
- `Level1Params::rs` adds the zero-default Berkeley external source resistance
  parameter with finite, non-negative validation.
- `Level1Params::rd` adds the zero-default Berkeley external drain resistance
  parameter with finite, non-negative validation.
- `Level1Params::tox` adds Berkeley-default gate oxide thickness and derives
  Meyer gate capacitance from `Cox = epsilon_ox / TOX`.
- `Level1Params::ld` applies Berkeley lateral-diffusion geometry through
  `L_eff = L - 2*LD` to channel current and length-scaled capacitance.
- `Level1Params::pb`, `Level1Params::mj`, and `Level1Params::fc` provide
  continuous Berkeley bulk-junction depletion-capacitance shaping for `CBS`
  and `CBD`.
- `Level1Params::af` provides the Level-1 MOS flicker-noise current exponent,
  defaulting to one.

## [0.1.0] — 2026-06-13

### Added

- `Level1Params` — 17-parameter SPICE Level-1 MOSFET parameter set with 130 nm NMOS defaults, including the zero-default `KF` flicker-noise coefficient.
- `Region` enum — `Cutoff`, `Subthreshold`, `Triode`, `Saturation` with `as_str()`.
- `MosResult` — complete small-signal result: `Id`, `gm`, `gds`, `gmb`, 5 capacitances, `region`.
- `evaluate_level1(params, V_GS, V_DS, V_BS, T)` — core Level-1 model:
  - Body effect via γ coefficient (clamped at heavy forward bias).
  - Subthreshold current via exp(V_OV / (n V_T)) when `subthreshold_enable = true`.
  - Channel-length modulation via λ in triode and saturation regions.
  - Meyer capacitance model: overlap (per-width CGSO/CGDO) + intrinsic (2/3 WL KP in saturation).
  - Body transconductance gmb via ∂V_t/∂V_BS.
- `MosfetType` enum — `Nmos` / `Pmos`.
- `Mosfet` struct — high-level wrapper; PMOS sign-flips input voltages and negates Id.
- `Level1Model` struct — compatibility wrapper for external model card parsers.
- 18 integration tests covering all regions, PMOS conventions, body effect, capacitance scaling, and the saturation Id formula.
