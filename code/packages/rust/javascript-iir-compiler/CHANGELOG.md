# Changelog

## Unreleased

- Select an explicit division-only source parser for the native numeric
  subset. Repeated division now lowers directly to IIR, including combined
  negative-zero, Infinity and NaN console fixtures. Retain regex rejection,
  source/AST/VM bounds and atomic display failure; general JavaScript
  lexical-goal disambiguation remains separate.
- Reject excessive flat operator/member/call spines before typed AST
  construction, avoiding recursive-drop stack overflow on hostile source.
  Preserve the existing 64-level semantic depth boundary and source limits.

- Accept exactly three numeric `console.log` arguments through direct typed
  AST-to-IIR lowering and Rust vm-core execution. Preserve source order,
  existing argument forms and budgets, and earlier completed output when
  number display fails in any argument; reject four or more arguments.
- Accept exactly two numeric `console.log` arguments through the typed AST
  and Rust VM builtin, formatting them with one space and one newline. Keep
  completed earlier output if a later call fails before appending its text.
- Accept zero-argument `console.log()` through the typed JavaScript AST and
  Rust VM builtin, appending exactly one newline without a numeric operand.
  Preserve that completed newline if a later accepted VM operation fails.
- Preserve completed JavaScript console output when a later VM instruction fails.

## 0.1.0

- Add a bounded direct JavaScript AST to IIR compiler and native VM runner.
- Bound source loading, typed AST lowering, and console output before allocation.
