# Changelog

## [0.1.0] — Unreleased

### Added
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
