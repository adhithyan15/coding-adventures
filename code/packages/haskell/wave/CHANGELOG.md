# Changelog

## Unreleased

- Enforce the full PHY01 finite-input and angular-frequency-overflow contract.
- Add full-range binary64 remainder reduction without host trigonometry,
  including positive subnormal frequencies with infinite represented periods.
- Make both build entrypoints run the complete package test suite.

## 0.1.0 - 2026-07-18

- Add validated sinusoidal waves with amplitude, frequency, and phase.
- Add period, angular-frequency, and time-evaluation operations.
- Build on the local first-principles `trig` package.
