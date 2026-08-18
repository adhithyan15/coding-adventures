# Changelog

All notable changes to the Ruby Parser package will be documented in this file.

## [0.1.3] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_ruby_parser` now imports a pre-compiled `_grammar` module instead of reading and parsing the `ruby.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published PyPI package does not ship, so `pip install` + first use would raise `FileNotFoundError`.
- **Stale grammar bug**: the pre-existing checked-in `src/ruby_parser/_grammar.py` was never imported by `parser.py` (which always read `ruby.grammar` directly from disk), so nobody noticed it had drifted drastically out of date — it only embedded 9 rules (`program`, `statement`, `assignment`, `expression_stmt`, `method_call`, and the `expression`/`term`/`factor` precedence chain) compiled from an early version of `ruby.grammar`. The current `code/grammars/ruby/ruby.grammar` defines ~97 rules and has grown to cover method definitions (`def`), classes (`class`/`module`), conditionals (`if`/`unless`/`elsif`/`else`), loops (`while`/`until`), `case`/`when`/`in` pattern matching, `begin`/`rescue`/`ensure` blocks, blocks, lambdas, and more — none of which the stale compiled file could parse. Regenerated fresh from the current grammar source; the core rule names exercised by the existing test suite (`program`, `assignment`, `method_call`, `expression`, `sum`, `term`, `factor`) were preserved, so all 21 pre-existing tests still pass unchanged.
- Added a `TestStatementLevelConstructs` regression test class (6 tests) covering `def`, `class`, `if`, `while`, `case`, and `begin` statements, so a future regression back to a stale/truncated grammar would be caught immediately.

## [0.1.2] - 2026-06-02

### Fixed

- Updated the operator-precedence tests to match the current `ruby.grammar`.
  The grammar now threads expressions through a full precedence chain
  (`expression → ternary → … → sum → term → factor`), so additive operators
  surface at the `sum` level rather than directly under `expression`. The
  `test_multiplication_before_addition` and `test_chained_addition` tests were
  still asserting the old flat shape (PLUS directly under `expression`) and
  failed once the package was rebuilt in CI. They now inspect the `sum` node.

## [0.1.1] - 2026-03-31

### Fixed

- Updated `ruby.grammar` so that `method_call` and `factor` reference `KEYWORD`
  instead of `PUTS`, `TRUE`, `FALSE`, and `NIL`. The grammar-driven lexer
  reclassifies all keyword identifiers (including `puts`, `true`, `false`, `nil`)
  to `KEYWORD` tokens; the grammar must use that token type.
  This fixes `GrammarParseError: Parse error at 1:1: Unexpected token: "puts"`.

## [0.1.0] - 2026-03-18

### Added
- Initial release of the Ruby parser package.
- `parse_ruby()` function that parses Ruby source code into a generic AST.
- `create_ruby_parser()` factory function for creating a `GrammarParser` configured for Ruby.
- Ruby parser grammar file (`ruby.grammar`) with support for:
  - Programs (sequences of statements)
  - Assignment statements (`x = expression`)
  - Method calls with arguments (`puts("hello")`)
  - Expression statements
  - Arithmetic expressions with operator precedence (`+`, `-`, `*`, `/`)
  - Parenthesized sub-expressions
  - Factors: number literals, string literals, variable names
- Comprehensive test suite with 80%+ coverage.
