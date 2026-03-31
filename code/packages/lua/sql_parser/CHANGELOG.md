# Changelog — coding-adventures-sql-parser

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the grammar-driven SQL parser.
- `M.parse(source)` — tokenizes SQL source, loads `sql.grammar`, runs
  `GrammarParser`, and returns the root `ASTNode` (rule_name `"program"`).
- `M.create_parser(source)` — returns an initialized `GrammarParser` for
  manual control (e.g., trace-mode debugging).
- `M.get_grammar()` — exposes the cached `ParserGrammar` for inspection.
- Grammar caching: `sql.grammar` is read from disk and parsed exactly once
  per process; all subsequent calls reuse the cached grammar.
- Path navigation via `debug.getinfo` + `dirname`/`up` helpers, consistent
  with the pattern established by `json_parser` and `toml_parser`.
- Full test suite (`tests/test_sql_parser.lua`) covering:
  - Module surface (VERSION, parse, create_parser, get_grammar)
  - SELECT * FROM table
  - SELECT with column list, WHERE, DISTINCT, ORDER BY, LIMIT, JOIN, GROUP BY
  - INSERT INTO … VALUES (…) — with and without column list
  - UPDATE … SET … WHERE … — single and multiple assignments
  - DELETE FROM … WHERE …
  - Expression nodes: comparison, column_ref, or_expr, and_expr, additive,
    function_call
  - Multiple semicolon-separated statements
  - Error handling: invalid SQL, incomplete SELECT, empty input
- `BUILD` and `BUILD_windows` with transitive dependency installation in
  leaf-to-root order: state_machine → directed_graph → grammar_tools →
  lexer → sql_lexer → parser → sql_parser.
- `required_capabilities.json` declaring `filesystem:read`.
- `README.md` with architecture description, grammar listing, and usage examples.
