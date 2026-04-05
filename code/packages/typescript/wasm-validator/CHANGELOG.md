# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added

- Initial TypeScript implementation of `wasm-validator`
- Structural validation for imports, exports, tables, memories, globals,
  start functions, element segments, data segments, and constant expressions
- Function-body decoding and abstract stack-machine type checking for W02 core
  control flow, variable access, memory operations, numeric instructions,
  conversions, calls, and unreachable code handling
- `ValidatedModule`, `ValidationError`, `ValidationErrorKind`, `validate`,
  `validateStructure`, `validateFunction`, and `validateConstExpr`
- Test coverage for representative valid modules and structural/type failures
