# Changelog

## [0.1.0] - 2026-04-12

### Added

- `TypeChecker[ASTIn, ASTOut]` Protocol — generic interface for all language
  type checkers in this repo.  Uses structural subtyping (`typing.Protocol`):
  no inheritance required.
- `TypeCheckResult[ASTOut]` frozen dataclass — carries the typed AST and a
  list of `TypeErrorDiagnostic` objects; exposes an `.ok` property as a
  shorthand for `len(errors) == 0`.
- `TypeErrorDiagnostic` frozen dataclass — represents one type error with
  `message`, `line`, and `column` source-location fields.
- Full test suite with ≥ 20 test cases covering construction, immutability,
  structural protocol satisfaction, duck typing, and the public API surface.
