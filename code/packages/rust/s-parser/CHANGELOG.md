# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-06-19

### Changed

- **R-19 grammar**: the `arg` rule now allows an **empty named-argument value** —
  `arg = NAME EQ [expr] | expr`. A named argument may omit its value (`a = ,` /
  `a = )`), which `switch`'s empty-arm fall-through relies on
  (`switch("a", a = , b = "hit")` → `"hit"`). The optional `expr` is only ever
  followed by `COMMA` or `RPAREN`, so the rule stays LL(1). This is a shared
  S/R grammar change; `r-parser` carries the mirror. Regenerated the embedded
  `src/_grammar.rs` (single-line functional change: `expr` → `Optional(expr)`).

## [0.2.0] - 2026-06-15

### Added

- `special` rule for `%op%` infix operators (left-associative, between `* /`
  and `:`).
- `dollar_suffix` (`df$name`) and `dindex_suffix` (`x[[k]]`) postfixes; 2-D
  `index_suffix` (`df[i, j]`) is now accepted.

### Changed

- Corrected the operator-precedence cascade so `:` binds tighter than `+ - * /`
  (matching R). Regenerated the embedded `_grammar.rs`.

## [0.1.0] - 2026-06-15

### Added

- Initial release of the historical Bell Labs S parser crate.
- `parse_s()` and `try_parse_s()` entry points returning a `GrammarASTNode`
  rooted at the `program` rule.
- `create_s_parser()` factory returning a configured `GrammarParser`.
- Embedded `s.grammar` (`src/_grammar.rs`), generated ahead of time.
- Full S v1 expression grammar with a precedence cascade: assignment
  (`<- _ <<- ->`, right-associative), comparison, the `:` sequence operator,
  additive/multiplicative arithmetic, prefix unary minus, right-associative
  `^`, and left-associative call/index postfixes.
- Function definitions with positional and default parameters; calls with
  positional and named (`name = expr`) arguments; `if`/`else`, `for`, `while`,
  and `repeat` as expressions; `{ }` blocks; and `( )` grouping.
- Statements separated by newlines or semicolons; calls and indices may span
  multiple physical lines.
