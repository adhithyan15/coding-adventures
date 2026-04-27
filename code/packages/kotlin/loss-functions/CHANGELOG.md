# Changelog — loss-functions (Kotlin)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of Kotlin loss-functions package.
- `LossFunctions.mse()` — Mean Squared Error.
- `LossFunctions.mae()` — Mean Absolute Error.
- `LossFunctions.bce()` — Binary Cross-Entropy with epsilon clamping.
- `LossFunctions.cce()` — Categorical Cross-Entropy with epsilon clamping.
- `LossFunctions.mseDerivative()` — MSE gradient.
- `LossFunctions.maeDerivative()` — MAE gradient.
- `LossFunctions.bceDerivative()` — BCE gradient.
- `LossFunctions.cceDerivative()` — CCE gradient.
- Input validation (empty arrays, length mismatch).
- JUnit 5 test suite with parity vectors from ML01 spec.
