# Changelog — coding-adventures-javascript-parser

## [0.2.0] — 2026-04-05

### Added

- `version` parameter added to `parse(source, version)`,
  `create_parser(source, version)`, and `get_grammar(version)`.
- Version routing: when `version` is `"es1"`, `"es3"`, `"es5"`, or
  `"es2015"` through `"es2025"`, the corresponding versioned grammar files
  are loaded from `code/grammars/ecmascript/<version>.grammar` and the
  lexer uses `code/grammars/ecmascript/<version>.tokens`.
- Generic fallback: passing `nil` or `""` loads the unified grammars as
  before (backward compatible).
- Per-version parser grammar cache keyed by version string.
- Validation: unknown version strings raise a descriptive error immediately.
- Extended test suite: new `describe("version-aware parsing")` block
  covering ES1/ES3/ES5/ES2015/ES2025 versions, `create_parser`, `get_grammar`,
  and error cases for invalid versions.

### Changed

- `M.VERSION` bumped from `"0.1.0"` to `"0.2.0"`.

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
