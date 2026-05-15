# Changelog

## [0.1.0] - 2026-05-14

### Added

- Added Rust operation envelope primitives: `OperationResult`, `ResultFactory`,
  `OperationScope`, `OperationOutcome`, `OperationError`, and `start_new`.
- Added panic capture with optional rethrow through `panic_on_unexpected`.
- Added `OperationHttpClient`, an operation-side HTTP preflight wrapper fed by
  generated code from `required_capabilities.json` that refuses undeclared HTTPS
  domains before transport callbacks can run.
