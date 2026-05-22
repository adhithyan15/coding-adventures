# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.2.0] - 2026-05-20

### Added (Phase 6a — `def name(params) … end` method definitions)
- New `def_statement` grammar rule in `code/grammars/ruby.grammar`:
  - `def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] { !"end" statement } "end"`
  - `params = NAME { COMMA NAME }`
- Added as the first alternative in `statement`, so `def`-led statements dispatch ahead of the `assignment` / `method_call` / `expression_stmt` branches.
- The body's `Repetition { statement }` uses a **negative lookahead** `!"end"` so it stops greedy-matching once the trailing `end` keyword comes into view.  Without the lookahead, `expression_stmt → factor → KEYWORD` would happily consume `end` itself as part of the body and leave nothing for the closing literal.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Tests (+4 new, total 10)
- `test_parse_def_no_params_no_body` — `def foo()\nend`.
- `test_parse_def_with_params` — `def add(x, y)\nend` produces a `params` subnode with names `["x", "y"]`.
- `test_parse_def_with_body` — `def add(x, y)\n  x + y\nend` carries at least one body `statement`.
- `test_parse_def_without_parens` — `def foo\nend` works (parens optional per Ruby).

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_parser(source)` — factory function that loads `ruby.grammar` and returns a configured `GrammarParser`.
- `parse_ruby(source)` — convenience function that parses Ruby source and returns a `GrammarASTNode`.
- Loads grammar from `ruby.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, expressions, method definitions, if/else, while loops, multiple statements, empty programs, class definitions, method calls, and the factory function.
