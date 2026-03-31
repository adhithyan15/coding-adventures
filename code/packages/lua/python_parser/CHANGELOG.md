# Changelog — coding-adventures-python-parser

## [0.1.0] — 2026-03-29

### Added
- Initial implementation of the grammar-driven Python parser.
- `parse(source)` — tokenizes with `python_lexer`, loads `python.grammar`,
  runs `GrammarParser`, and returns the root `ASTNode`.
- `create_parser(source)` — returns an initialized `GrammarParser` without
  immediately parsing, for trace-mode or custom parsing workflows.
- `get_grammar()` — returns the cached `ParserGrammar` for inspection.
- Grammar-file caching: `python.grammar` is loaded once and reused.
- Supports assignments (`x = 5`), arithmetic with correct operator precedence
  (`+`/`-` at expression level, `*`/`/` at term level), parenthesized groups,
  and expression statements.
- Full busted test suite in `tests/test_python_parser.lua` covering:
  module API, root node structure, assignments, expression statements,
  operator precedence, multiple statements, grammar inspection, and
  error handling.
- `required_capabilities.json` declaring `filesystem:read` capability.
- `BUILD` and `BUILD_windows` for the monorepo build system.
- `README.md` with usage examples, grammar listing, and stack diagram.
