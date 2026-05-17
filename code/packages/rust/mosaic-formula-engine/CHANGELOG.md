# Changelog — mosaic-formula-engine

All notable changes to this crate are recorded here.
Versioning follows [Semantic Versioning](https://semver.org/).

---

## [0.1.0] — 2026-05-16

### Added

Initial release implementing spec FE01.

**Core types**
- `CellAddr` — parse and format cell addresses (A1–Z99)
- `CellValue` — `Empty`, `Text`, `Number`, `Bool`, `Error` variants
- `FormulaError` — `DivZero`, `Ref`, `Name`, `Value`, `Circ`, `Parse` with
  VisiCalc-style display strings (`#DIV/0!`, `#REF!`, `#NAME?`, `#VALUE!`,
  `#CIRC`, `#PARSE`)

**Formula engine**
- `FormulaEngine::new()` — create an empty engine
- `FormulaEngine::set_raw()` — store a literal or formula string
- `FormulaEngine::get_display()` — read the computed cell value string
- `FormulaEngine::get_formula()` — read the raw formula string
- `FormulaEngine::recalculate()` — evaluate all dirty cells in
  topological order using Kahn's algorithm; detect circular dependencies

**Formula language**
- Arithmetic operators: `+`, `-`, `*`, `/`
- Unary negation: `-expr`
- Parenthesised sub-expressions: `(expr)`
- Cell references: `A1`–`Z99`
- Range references inside function arguments: `A1:C3`
- String literals: `"hello"`
- Boolean literals: `TRUE`, `FALSE`
- Six built-in functions: `SUM`, `AVG`, `COUNT`, `MAX`, `MIN`, `IF`

**Internal modules**
- `lexer` — single-pass byte scanner, produces `Vec<Token>`
- `parser` — recursive-descent parser, produces `Expr` AST
- `eval` — AST evaluator with error propagation and range expansion
- `builtins` — pure implementations of the six built-in functions
- `graph` — directed dependency graph with topological sort and cycle detection

**Quality**
- 73 unit tests covering all 29 spec-required test cases plus module-level
  unit tests
- Zero external dependencies (only `std`)
- Zero `unsafe` blocks
- Zero clippy warnings (`-D warnings`)
- Literate programming style throughout
