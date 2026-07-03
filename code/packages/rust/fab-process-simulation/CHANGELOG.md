# Changelog — fab-process-simulation

## [0.1.0] — 2026-06-13

### Added

- `Layer` struct — material name, thickness_nm, doping map (`species → [(depth_nm, conc/cm³)]`).
- `CrossSection` struct — ordered list of layers, top-to-bottom.
- `deal_grove_oxidation(cs, time_min, A?, B?)` — Deal-Grove quadratic growth law with τ correction for pre-existing oxide.
- `deposit(cs, material, thickness_nm)` — prepend a uniform film layer.
- `etch(cs, target_material, depth_nm)` — layer-selective depth removal from the top.
- `implant(cs, species, energy_keV, dose_per_cm2)` — Gaussian profile from SRIM table; linear interpolation between tabulated energies.
- `diffuse(cs, time_min, temperature_C?)` — Fick's law Gaussian broadening (simplified v0.1 model).
- `implant_range(species, energy_keV)` — SRIM table lookup with linear interpolation and extrapolation.
- `diffusivity_cm2_per_s(species, temperature_C)` — Arrhenius T²-scaled diffusivity from 1000 °C reference.
- `implant_range_table()` — 9-entry SRIM table for B, P, As, BF2.
- `diffusivity_1000c(species)` — reference diffusivity at 1000 °C [cm²/s].
- 24 integration tests covering all process steps, edge cases, and error paths.
