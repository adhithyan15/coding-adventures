# Changelog — coding-adventures-ruby-parser

## Unreleased

### Changed

- `_grammar` regenerated from `code/grammars/ruby/ruby.grammar` (#18778),
  which gained a `for_statement` (`for x in xs [do] … end`), `rescue => e`
  with no class, a guard so a parenthesised call followed by a binary operator
  (`f() + f`) parses as one expression statement, and a lookahead that keeps a
  `[` re-tagged `lbracket_line_start` (only the Rust `ruby-lexer` produces it)
  from being an index or a paren-less call's first argument.  This package's
  lexer never produces that value, so the last change does not affect it here.

### Fixed

- Eliminated runtime grammar loading: `get_grammar` now requires a
  pre-compiled `_grammar` module instead of reading and parsing the
  `ruby.grammar` file from `code/grammars/` on every call. The old code
  walked out of the installed package's own directory to a
  monorepo-relative path that a published LuaRocks package does not ship.
- Verified against the current `ruby.grammar` (which now defines ~97
  rules, up from the version this package was originally written
  against) that `def`/`class`/`if`/`while`/`case`/`begin` statements all
  parse correctly; added regression tests covering each.

## [0.1.0] — 2026-03-29

### Added
- Initial implementation of the grammar-driven Ruby parser.
- `parse(source)` — tokenizes with `ruby_lexer`, loads `ruby.grammar`,
  runs `GrammarParser`, and returns the root `ASTNode`.
- `create_parser(source)` — returns an initialized `GrammarParser` without
  immediately parsing, for trace-mode or custom parsing workflows.
- `get_grammar()` — returns the cached `ParserGrammar` for inspection.
- Grammar-file caching: `ruby.grammar` is loaded once and reused.
- Supports assignments (`x = 5`), method calls (`puts("hello")`),
  arithmetic with correct operator precedence (`+`/`-` at expression level,
  `*`/`/` at term level), parenthesized groups, keyword expressions
  (`true`, `false`, `nil`), and expression statements.
- Full busted test suite in `tests/test_ruby_parser.lua` covering:
  module API, root node structure, assignments, method calls,
  expression statements, operator precedence, multiple statements,
  grammar inspection, and error handling.
- `required_capabilities.json` declaring `filesystem:read` capability.
- `BUILD` and `BUILD_windows` for the monorepo build system.
- `README.md` with usage examples, grammar listing, and stack diagram.
