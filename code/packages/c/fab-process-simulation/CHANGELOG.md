# Changelog

All notable changes to the C `fab-process-simulation` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `fab-process-simulation` crate — a 1-D
  analytical CMOS process-flow simulator.
- `FabCrossSection` layer stack (with per-layer, per-species doping profiles)
  and the process steps `fab_deal_grove_oxidation` (sqrt), `fab_deposit`,
  `fab_etch` (material-selective, top-down), `fab_implant` (Gaussian profile via
  `exp`), and `fab_diffuse` (v0.1.0 preserves samples); plus `fab_implant_range`
  (SRIM lookup + interpolation/extrapolation) and `fab_diffusivity_cm2_per_s`
  (Arrhenius T² scaling).
- Every step deep-copies its input (never mutating it); allocations guard
  reallocation against `size_t` overflow; the `sqrt` (Newton) and `exp`
  (Cody-Waite) are computed without `<math.h>`.
- `FabStatus` status-code API in place of the Rust `Result<_, String>`.
- 58 checks against reference values captured from the real Rust crate (an
  oracle run: Deal-Grove thicknesses, implant-range interpolation, Gaussian
  peak concentration, Arrhenius diffusivity), run under every available C
  compiler via the shared `iso-harness`; the suite also passes under ASan +
  UBSan.
