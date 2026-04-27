# Changelog — tetrad-type-checker

## [0.1.0] — 2026-04-20

### Added

- `tetrad_type_checker.types` module: `TypeInfo`, `FunctionType`, `TypeEnvironment`, `FunctionTypeStatus`, `TypeError`, `TypeWarning`, `TypeCheckResult` dataclasses
- `FunctionTypeStatus` enum: FULLY_TYPED, PARTIALLY_TYPED, UNTYPED
- Four-phase type-checking algorithm: signature collection → global checking → function body checking → classification
- Bottom-up expression type inference: u8 literals, u8 variable lookup, binary ops (u8 × u8 → u8), comparisons (always u8), logical ops (always u8), unary ops (! and ~ always u8; - preserves operand type), call expressions (result type = callee return type), in() → Unknown, out() → Void, GroupExpr → transparent
- Function classification: FULLY_TYPED when all params+return annotated and all body ops infer u8; PARTIALLY_TYPED when partial annotations or ops yield Unknown; UNTYPED when no annotations
- Hard errors for: let x: u8 = in() (assigning Unknown to annotated), return in() from → u8 function
- Soft warnings for: untyped functions (JIT warmup required), typed function calling untyped callee (downgraded to PARTIALLY_TYPED)
- `check(program) -> TypeCheckResult` and `check_source(source) -> TypeCheckResult` public entry points
- 80+ unit tests, 95%+ line coverage
