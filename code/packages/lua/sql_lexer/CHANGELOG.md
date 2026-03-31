# Changelog — coding-adventures-sql-lexer (Lua)

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-03-29

### Added

- Initial implementation of the SQL lexer Lua package.
- `tokenize(source)` — tokenizes a SQL source string using the grammar-driven
  `GrammarLexer` infrastructure, returning a flat list of typed tokens.
- `get_grammar()` — returns the cached `TokenGrammar` object parsed from
  `sql.tokens`, for callers that want to inspect or reuse the grammar.
- Grammar loading with caching — the `sql.tokens` file is read and parsed once
  per process; subsequent calls reuse the cached `TokenGrammar`.
- Path navigation — locates `sql.tokens` by walking 6 directories up from the
  module file to the `code/` repo root, then descending into `grammars/`.
- Full test suite (`tests/test_sql_lexer.lua`) covering:
  - Module surface (VERSION, tokenize, get_grammar)
  - Empty and trivial inputs (empty, whitespace-only, comment-only)
  - SELECT * FROM users WHERE id = 1
  - SELECT with column list, comparison operators, ORDER BY, LIMIT, DISTINCT
  - Case-insensitive keyword matching (select → SELECT)
  - INSERT INTO ... VALUES ... with column list
  - UPDATE ... SET ... WHERE ...
  - DELETE FROM ... WHERE ...
  - Single-quoted string literals
  - Integer and decimal numeric literals
  - NULL, TRUE, FALSE literals
  - All comparison operators: =, !=, <>, <, >, <=, >=
  - All arithmetic operators: +, -, *, /, %
  - Delimiters: ( ) , ; .
  - Line comments (-- ...) and block comments (/* ... */)
  - JOIN clauses (INNER JOIN, LEFT JOIN)
  - BETWEEN...AND, LIKE, IN list
  - GROUP BY, HAVING, ORDER BY
  - CREATE TABLE statement
  - Token position tracking (line, col)
  - Error on unexpected character
- `coding-adventures-sql-lexer-0.1.0-1.rockspec` rockspec with correct
  transitive dependencies (state-machine, directed-graph, grammar-tools, lexer).
- `BUILD` and `BUILD_windows` scripts installing all dependencies leaf-to-root.
- `required_capabilities.json` declaring `filesystem:read` capability.
