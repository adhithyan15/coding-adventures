# Changelog — coding_adventures_activation_functions

## 0.1.0 — 2026-07-11

### Added

- Initial release: pure-Dart port of the `activation-functions` reference
  package.
- Activations with matching derivatives: `linear`/`linearDerivative`,
  `sigmoid`/`sigmoidDerivative`, `relu`/`reluDerivative`,
  `leakyRelu`/`leakyReluDerivative`, `tanh`/`tanhDerivative`,
  `softplus`/`softplusDerivative`, plus the `leakyReluSlope` constant.
- Numerically stable `tanh` and `log1p` (absent from `dart:math`) and overflow
  guards on `sigmoid`/`softplus`, matching the Rust `f64` semantics.
- 17 unit tests asserting the crate's reference values, extreme-value
  saturation, and a tanh sweep against the exponential definition.
