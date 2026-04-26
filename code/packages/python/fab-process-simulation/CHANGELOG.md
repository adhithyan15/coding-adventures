# Changelog

## [0.1.0] — Unreleased

### Added
- `CrossSection` + `Layer` data classes for vertical wafer stacks with per-species doping profiles.
- `deal_grove_oxidation(cs, time_min)`: Deal-Grove thermal SiO2 growth on Si. Default constants for dry O2 at 1000°C.
- `deposit(cs, material, thickness_nm)`: uniform film deposition.
- `etch(cs, target_layer, depth_nm)`: anisotropic etch of top layer if material matches.
- `implant(cs, species, energy_keV, dose_per_cm2)`: Gaussian profile from SRIM-derived IMPLANT_RANGES table (B, P, As, BF2 at typical energies). Linear interpolation between tabulated values.
- `diffuse(cs, time_min, temperature_C)`: Fick's-law broadening with Arrhenius-scaled diffusivity.
- `IMPLANT_RANGES` and `DIFFUSIVITY_1000C` constants exposed.

### Out of scope (v0.2.0)
- 2-D/3-D TCAD PDE solvers.
- Hopkins-formula lithography simulation.
- Pattern density-dependent CMP.
- Stress engineering, FinFET flows, BEOL via simulation.
