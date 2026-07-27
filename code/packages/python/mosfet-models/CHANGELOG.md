# Changelog

## [0.1.0] — Unreleased

### Added
- `Level1Params.RS` adds the zero-default Berkeley external source resistance
  parameter with finite, non-negative validation.
- `Level1Params.RD` adds the zero-default Berkeley external drain resistance
  parameter with finite, non-negative validation.
- `Level1Params.TOX` adds Berkeley-default gate oxide thickness and derives
  Meyer gate capacitance from `Cox = epsilon_ox / TOX`.
- `Level1Params.LD` applies Berkeley lateral-diffusion geometry through
  `L_eff = L - 2*LD` to channel current and length-scaled capacitance.
- `Level1Params.FC` adds the continuous Berkeley forward-bias continuation to
  the existing `PB` / `MJ` bulk-junction depletion-capacitance model.
- `Level1Params.AF` provides the Level-1 MOS flicker-noise current exponent,
  defaulting to one.
- `Level1Params.KF` provides the Level-1 MOS flicker-noise coefficient, with a
  noise-disabled default of zero.
- `Level1Params` now includes MOS Level-1 capacitance footholds (`CGSO`,
  `CGDO`, `CGBO`, `CBS`, `CBD`) and reports them through `MosResult`.
- `Level1Params.PB` and `Level1Params.MJ` now shape zero-bias bulk-junction
  `CBS`/`CBD` capacitance under reverse source-bulk and drain-bulk bias.
- `Level1Params`: SPICE Level-1 parameter set (VT0, KP, LAMBDA, GAMMA, PHI, W, L, IS, N_SUB, T_NOM, subthreshold_enable).
- `evaluate_level1(params, V_GS, V_DS, V_BS, T)`: classical square-law I-V with body effect, channel-length modulation, and optional subthreshold current.
- `MosResult`: Id + small-signal Jacobian (gm, gds, gmb) + Meyer capacitances + region label.
- `MosfetModel` Protocol: common `dc(V_GS, V_DS, V_BS, T) -> MosResult` interface.
- `Level1Model`: dataclass implementing MosfetModel.
- `MOSFET(type, model)` wrapper: NMOS/PMOS unification by sign-flipping for PMOS.
- Region detection: cutoff / subthreshold / triode / saturation.

### Out of scope (v0.2.0)
- EKV (smooth all-region).
- BSIM3v3 subset for Sky130 characterization.
- Velocity saturation.
- Non-quasi-static dynamic model.
- Aging (NBTI/PBTI/HCI).
