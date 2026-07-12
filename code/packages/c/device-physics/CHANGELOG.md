# Changelog

All notable changes to the C `device-physics` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `device-physics` crate — closed-form
  semiconductor device-physics models in SI units.
- The physical constants, `dp_thermal_voltage`, `dp_intrinsic_concentration`
  (with `exp`), and `dp_fermi_potential` (with `ln`).
- `DpPNJunction` (`built_in_voltage`, `depletion_width`, `saturation_current`,
  Shockley `current`) and `DpMOSFET` (`c_ox`, `v_fb`, `phi_f`, `gamma`,
  `threshold_voltage` with body effect), built via checked `*_new` constructors.
- `exp`, `ln`, and `sqrt` computed without `<math.h>`; `powf(x, 1.5)` as
  `x·sqrt(x)`. `DpStatus` status-code API in place of the Rust
  `Result<_, String>`.
- The from-scratch `ln` guards non-finite / non-positive arguments before its
  range-reduction loops, so an overflowed intermediate (e.g. `na*nd` reaching
  `+inf`) yields a sentinel rather than looping forever.
- 39 checks against reference values captured from the real Rust crate (an
  oracle run: thermal voltage, n_i(T), Fermi/built-in potentials, depletion
  width, saturation & diode current, MOSFET threshold with body effect) plus
  overflow-termination regressions, run under every available C compiler via the
  shared `iso-harness`; the suite also passes under UBSan.
