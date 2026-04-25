# Changelog

## [0.1.0] — Unreleased

### Added
- Constants: K_BOLTZMANN, Q_ELECTRON, EPS0, EPS_SI, EPS_OX, N_I_300K, N_C, N_V, EG_SI_300K, MU_N_300K, MU_P_300K.
- `thermal_voltage(T)`: V_T = kT/q.
- `intrinsic_concentration(T)`: n_i(T) with standard temperature scaling.
- `fermi_potential(N, kind, T)`: phi_F for p-type or n-type.
- `PNJunction(N_A, N_D, A, T, tau_n, tau_p)`: built-in voltage, depletion width, saturation current, Shockley diode current.
- `MOSFETParams(type, L, W, T_ox, N_body, phi_MS, Q_ox, T)`: derived properties C_ox, V_FB, phi_F, gamma; `threshold_voltage(V_SB=0)` with body effect.
- Validates Sky130-style 130 nm NMOS V_t ≈ 0.42 V from physical parameters.

### Out of scope (v0.2.0)
- Quantum corrections (polysilicon depletion, channel quantization).
- Bandgap narrowing.
- Gate tunneling, GIDL, BTBT.
- Aging models (NBTI, PBTI, HCI).
- High-K / metal-gate parameter sets.
