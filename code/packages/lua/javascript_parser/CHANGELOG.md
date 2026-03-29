# Changelog — coding-adventures-javascript-parser

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the grammar-driven JavaScript parser.
- `M.parse(source)` — tokenizes JavaScript source, loads `javascript.grammar`,
  runs `GrammarParser`, and returns the root `ASTNode` (rule_name `"program"`).
- `M.create_parser(source)` — returns an initialized `GrammarParser` for
  manual control (e.g., trace-mode debugging).
- `M.get_grammar()` — exposes the cached `ParserGrammar` for inspection.
- Grammar caching: `javascript.grammar` is read from disk and parsed exactly
  once per process; all subsequent calls reuse the cached grammar.
- Path navigation via `debug.getinfo` + `dirname`/`up` helpers, consistent
  with the pattern established by `json_parser`, `toml_parser`, and `sql_parser`.
- Full test suite (`tests/test_javascript_parser.lua`) covering:
  - Module surface (VERSION, parse, create_parser, get_grammar)
  - Variable declarations with `var`, `let`, and `const`
  - Assignments: `x = 10;`
  - Expression statements: `42;`  `x;`
  - Expression precedence: `1 + 2 * 3` produces correct tree layering
  - Parenthesized expressions: `(2 + 3) * 4`
  - Multiple statements in one parse call
  - Empty program (zero statements)
  - Error handling: invalid input raises an error
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state_machine → directed_graph → grammar_tools →
  lexer → javascript_lexer → parser → javascript_parser.
- `required_capabilities.json` declaring `filesystem:read`.
- `README.md` with architecture description, grammar listing, operator
  precedence explanation, and usage examples.
