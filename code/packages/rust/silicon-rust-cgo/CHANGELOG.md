# Changelog — silicon-rust-cgo

## [0.1.0] — 2026-06-13

### Added

Initial release.  Plain C ABI cdylib for the silicon simulation stack.

**28 exported C symbols:**

*Physical constants (9, infallible):*
`silicon_k_boltzmann`, `silicon_q_electron`, `silicon_eps0`,
`silicon_eps_si`, `silicon_eps_ox`, `silicon_ni_at_300k`,
`silicon_eg_si_at_300k`, `silicon_mu_n_300k`, `silicon_mu_p_300k`

*device-physics (9, 1 infallible + 8 fallible):*
`silicon_thermal_voltage` (infallible),
`silicon_intrinsic_concentration`,
`silicon_fermi_potential`,
`silicon_pn_junction_built_in_voltage`,
`silicon_pn_junction_depletion_width`,
`silicon_pn_junction_saturation_current`,
`silicon_pn_junction_current`,
`silicon_mosfet_threshold_voltage`

*mosfet-models (2, + `SiliconMosResult` struct):*
`silicon_evaluate_level1`, `silicon_evaluate_level1_defaults`

*fab-process-simulation (10, 1 infallible + 9 fallible):*
`silicon_deposit`, `silicon_etch`, `silicon_implant`,
`silicon_diffuse`, `silicon_diffuse_with_temp`,
`silicon_deal_grove_oxidation`, `silicon_deal_grove_oxidation_custom`,
`silicon_implant_range`, `silicon_diffusivity_cm2_per_s` (infallible)

**`include/silicon_cgo.h`** — C header shared with the Go CGo wrapper.

**Wire-format injection guard** — `silicon_deposit`, `silicon_etch`,
`silicon_implant` reject material/species names containing `|` or `:`.

**No undefined symbols** — all Rust dependencies are statically linked.
No platform-specific `build.rs` flags required.

**6 unit tests** covering wire-format helpers and name validation.
