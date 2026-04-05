# Changelog — loss-functions (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of Swift loss-functions package.
- `LossFunctions.mse(_:_:)` — Mean Squared Error.
- `LossFunctions.mae(_:_:)` — Mean Absolute Error.
- `LossFunctions.bce(_:_:)` — Binary Cross-Entropy with epsilon clamping.
- `LossFunctions.cce(_:_:)` — Categorical Cross-Entropy with epsilon clamping.
- `LossFunctions.mseDerivative(_:_:)` — MSE gradient.
- `LossFunctions.maeDerivative(_:_:)` — MAE gradient.
- `LossFunctions.bceDerivative(_:_:)` — BCE gradient.
- `LossFunctions.cceDerivative(_:_:)` — CCE gradient.
- Input validation (empty arrays, length mismatch).
- Comprehensive test suite with parity vectors from ML01 spec.
