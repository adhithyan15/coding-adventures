# Changelog — iir-type-checker

## [0.1.0] — 2026-05-04

### Added

- `tier::TypingTier` enum — `Untyped`, `Partial(f32)`, `FullyTyped` with
  `from_fraction()`, `is_fully_typed()`, `is_untyped()`, `typed_fraction()`,
  and `Display` implementations.
- `errors::TypeCheckError` and `TypeCheckWarning` with `ErrorKind` enum
  (`InvalidType`, `TypeMismatch`, `ConditionNotBool`).
- `report::TypeCheckReport` — bundles tier, typed_fraction, errors, warnings,
  and inferred_types map.  Provides `ok()` and `summary()` helpers.
- `check::check_module()` — read-only validation pass over an `IIRModule`:
  validates `type_hint` strings, binary-op operand consistency, and branch
  condition types.
- `infer::infer_types_mut()` — mutating SSA-propagation inference pass:
  R1–R8 rules for const literals, comparisons, arithmetic, bitwise, unary,
  and SSA-copy instructions.  Multi-pass fixed-point convergence.
- `infer_and_check()` — top-level convenience function: runs inference then
  checking, returning a unified `TypeCheckReport` with `inferred_types` populated.
- Comprehensive inline unit tests for all modules (60+ assertions).
