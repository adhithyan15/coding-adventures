# Changelog

## [0.1.0] - 2026-03-22

### Added

- Three-pass AST transformer: symbol collection, expansion, cleanup
- Lexical scope chain with parent-chain variable lookup and shadowing
- Compile-time expression evaluator supporting 9 value types
  (Number, Dimension, Percentage, String, Ident, Color, Bool, Null, List)
- Arithmetic on numbers, dimensions (same-unit), and percentages
- Comparison operators: `==`, `!=`, `>`, `>=`, `<=`
- Logical operators: `and`, `or`
- Variable expansion with `$name` references
- Mixin definitions (`@mixin`) and expansion (`@include`) with parameters
- Function definitions (`@function`) and evaluation with `@return`
- Control flow: `@if`/`@else`, `@for` loops, `@each` iteration
- Cycle detection for circular mixin/function references
- CSS emitter with pretty-print and minified output modes
- 10 structured error types with line/column position info
- All errors inherit from `LatticeError` for unified error handling
