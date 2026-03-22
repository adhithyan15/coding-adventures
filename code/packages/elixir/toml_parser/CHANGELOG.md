# Changelog

## 0.1.0 — 2026-03-21

### Added
- `TomlParser.parse/1` — parse TOML source code into an AST
- `TomlParser.create_parser/0` — parse toml.grammar
- Grammar caching via `persistent_term` for repeated use
- 43 tests covering grammar loading, key-value primitives (string, integer, float, boolean, datetime), key types (bare, dotted, quoted, literal, integer-as-key, true-as-key), table headers, array-of-tables headers, arrays (empty, elements, trailing comma, multi-line, nested, mixed types), inline tables (empty, key-value, nested), document structure (empty, comment-only, multiple expressions, blank lines), realistic TOML documents, error cases, and ASTNode helpers
