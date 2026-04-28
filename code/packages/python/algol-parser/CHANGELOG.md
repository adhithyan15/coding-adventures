# Changelog — coding-adventures-algol-parser

All notable changes to this package are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Unified `expression` parsing now accepts ALGOL conditional expressions such
  as `if b then x else y` in assignment values and actual parameters.
- Block and compound statement lists now tolerate repeated and trailing
  semicolons, matching common ALGOL dummy-statement style.

## [0.1.0] — 2026-04-06

### Added

- `parse_algol(source: str) -> ASTNode` — main entry point; parses ALGOL 60
  source text into a generic `ASTNode` parse tree rooted at `program`.
- `create_algol_parser(source: str) -> GrammarParser` — factory function for
  callers who want direct control over the parser object.
- Grammar-driven implementation using `algol.grammar` and the `GrammarParser`
  engine from `coding-adventures-parser`.
- Full ALGOL 60 grammar coverage:
  - **Top level**: `program` → `block`
  - **Block structure**: `BEGIN { declaration SEMICOLON } statement { SEMICOLON statement } END`
    enforces declaration-before-statement ordering at the grammar level.
  - **Declarations**: `type_decl`, `array_decl` (with dynamic bounds),
    `switch_decl`, `procedure_decl` (with `value`/`spec` parts)
  - **Statements**: `assign_stmt` (chainable `:=`), `cond_stmt`
    (dangling-else-free), `for_stmt` (step/until, while, simple),
    `goto_stmt`, `proc_stmt`, `compound_stmt`, `empty_stmt`
  - **Arithmetic expressions**: full 5-level precedence hierarchy
    (`arith_expr` → `simple_arith` → `term` → `factor` → `primary`),
    conditional arithmetic expressions (`if b then x else y`),
    left-associative exponentiation (`**`, `^`)
  - **Boolean expressions**: full 5-level precedence hierarchy
    (`bool_expr` → `simple_bool` → `implication` → `bool_term` →
    `bool_factor` → `bool_secondary` → `bool_primary`), with `relation`
    comparing arithmetic expressions
  - **Designational expressions**: `desig_expr` for computed goto targets
    (direct labels, switch subscripts, conditional jumps)
  - **Variables and calls**: subscripted array access (`A[i, j]`),
    procedure calls in expressions (`sqrt(x)`)
- Comprehensive pytest test suite covering:
  - Factory function and AST root structure
  - Minimal program parsing
  - Assignment statements (simple, real, expression)
  - Arithmetic expression structure and precedence
  - If/then, if/then/else, relational and boolean conditions
  - For loop (step/until form, compound body)
  - Nested block structure (2+ levels deep)
  - Boolean expression operators (AND, NOT, TRUE, FALSE)
  - Type declarations (integer, real, boolean, multiple variables)
  - Procedure calls as statements (no args, with args)
  - Compound statements (as if-branch and for-body)
  - Multiple sequential statements
  - Error cases (missing END, missing BEGIN, empty input,
    statement before declaration)
