# Changelog — coding-adventures-loss-functions (Lua)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-03-29

### Added

- `mse/2` — Mean Squared Error: `(1/n) * Σ(y - ŷ)²`
- `mae/2` — Mean Absolute Error: `(1/n) * Σ|y - ŷ|`
- `bce/2` — Binary Cross-Entropy with epsilon clamping to avoid `log(0)`
- `cce/2` — Categorical Cross-Entropy with epsilon clamping
- `mse_derivative/2` — analytical gradient `(2/n)(ŷ - y)`
- `mae_derivative/2` — subgradient `±1/n` (0 at exact match)
- `bce_derivative/2` — analytical gradient `(1/n)(p-y)/(p(1-p))`
- `cce_derivative/2` — analytical gradient `-(1/n)(y/p)`
- Input validation: both arrays must be non-empty tables of equal length
- `VERSION = "0.1.0"`
- Rockspec `coding-adventures-loss-functions-0.1.0-1.rockspec`
- Comprehensive busted test suite with numerical gradient checks
