# Changelog

All notable changes to this project will be documented in this file.

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
