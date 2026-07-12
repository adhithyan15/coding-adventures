# Changelog

All notable changes to the C++ `fab-process-simulation` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `fab-process-simulation`
  crate (namespace `ca::fab_process_simulation`) — a 1-D analytical CMOS
  process-flow simulator.
- Value-semantic `CrossSection` (`std::vector<Layer>`, each `Layer` with a
  `std::unordered_map` doping profile) and the process steps
  `deal_grove_oxidation` (sqrt), `deposit`, `etch`, `implant` (Gaussian via
  `exp`), and `diffuse`; plus `implant_range` (SRIM lookup +
  interpolation/extrapolation) and `diffusivity_cm2_per_s` (Arrhenius T²).
- `sqrt` (Newton) and `exp` (Cody-Waite) computed without `<cmath>`; bad steps
  throw `std::invalid_argument` in place of the Rust `Result<_, String>`.
- 33 checks against reference values captured from the real Rust crate (an
  oracle run), run under every available C++ compiler via the shared
  `iso-harness`.
