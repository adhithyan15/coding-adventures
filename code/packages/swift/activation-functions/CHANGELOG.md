# Changelog — activation-functions (Swift)

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-04

### Added
- Initial implementation of Swift activation-functions package.
- `ActivationFunctions.sigmoid(_:)` — sigmoid with overflow protection.
- `ActivationFunctions.sigmoidDerivative(_:)` — sigmoid gradient.
- `ActivationFunctions.relu(_:)` — Rectified Linear Unit.
- `ActivationFunctions.reluDerivative(_:)` — ReLU gradient.
- `ActivationFunctions.tanh(_:)` — hyperbolic tangent.
- `ActivationFunctions.tanhDerivative(_:)` ��� tanh gradient.
- Comprehensive test suite with parity vectors from ML04 spec.
