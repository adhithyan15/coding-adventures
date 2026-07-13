# Changelog

All notable changes to the C++ `device-physics` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `device-physics` crate
  (namespace `ca::device_physics`).
- Physical constants, `thermal_voltage`, `intrinsic_concentration` (exp),
  `fermi_potential` (ln); the `PNJunction` class (built-in voltage, depletion
  width, saturation & Shockley diode current) and the `MOSFETParams` class
  (c_ox, v_fb, phi_f, gamma, threshold_voltage with body effect).
- `exp`/`ln`/`sqrt` computed without `<cmath>`; bad inputs throw
  `std::invalid_argument` / `std::domain_error` in place of the Rust
  `Result<_, String>`.
- The from-scratch `ln` guards non-finite / non-positive arguments before its
  range-reduction loops, so an overflowed intermediate (e.g. `na*nd` reaching
  `+inf`) yields a sentinel rather than looping forever.
- 28 checks against reference values captured from the real Rust crate (an
  oracle run) plus an overflow-termination regression, run under every available
  C++ compiler via the shared `iso-harness`.
