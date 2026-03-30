# Changelog — CodingAdventures::LossFunctions (Perl)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.01] — 2026-03-29

### Added

- `mse($y_true, $y_pred)` — Mean Squared Error
- `mae($y_true, $y_pred)` — Mean Absolute Error
- `bce($y_true, $y_pred)` — Binary Cross-Entropy with epsilon clamping
- `cce($y_true, $y_pred)` — Categorical Cross-Entropy with epsilon clamping
- `mse_derivative($y_true, $y_pred)` — analytical gradient `(2/n)(ŷ - y)`
- `mae_derivative($y_true, $y_pred)` — subgradient `±1/n`
- `bce_derivative($y_true, $y_pred)` — analytical gradient `(1/n)(p-y)/(p(1-p))`
- `cce_derivative($y_true, $y_pred)` — analytical gradient `-(1/n)(y/p)`
- Input validation: both arguments must be array references of equal non-zero length
- `Makefile.PL` and `cpanfile` for CPAN-style packaging
- `t/00-load.t` and `t/01-basic.t` with numerical gradient checks and gradient-descent consistency tests
