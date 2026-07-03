# Changelog — device-physics

## [0.1.0] — 2026-06-13

### Added

- Physical constants: `K_BOLTZMANN`, `Q_ELECTRON`, `EPS0`, `EPS_SI`, `EPS_OX`, `N_I_300K`, `N_C`, `N_V`, `EG_SI_300K`, `MU_N_300K`, `MU_P_300K`.
- `thermal_voltage(T)` — kT/q, linear in temperature.
- `intrinsic_concentration(T)` — T^(3/2) × exp(−Eg/2kT) scaling from 300 K reference; returns error below 100 K.
- `fermi_potential(N, kind, T)` — φ_F for p-type and n-type silicon.
- `PNJunction` struct — `built_in_voltage()`, `depletion_width(V_applied)`, `saturation_current()`, `current(V)` (Shockley equation).
- `MOSFETParams` struct — `c_ox()`, `v_fb()`, `phi_f()`, `gamma()`, `threshold_voltage(V_SB)` with body effect.
- 20 unit tests covering all exported functions and error paths.
