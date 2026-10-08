# Changelog

## Unreleased

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
