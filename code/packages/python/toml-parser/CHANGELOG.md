# Changelog

All notable changes to the `coding-adventures-toml-parser` package.

## [0.1.0] - 2026-03-21

### Added

- `parse_toml(source)` — all-in-one function: parse TOML text → Python dict
- `parse_toml_ast(source)` — syntax-only parsing → generic ASTNode tree
- `convert_ast(ast)` — semantic validation + value conversion → TOMLDocument
- `create_toml_parser(source)` — factory for GrammarParser configured for TOML
- `TOMLDocument` — dict subclass representing a TOML document
- `TOMLConversionError` — raised for semantic constraint violations
- Two-phase parsing: context-free grammar parse → semantic validation pass
- Full TOML v1.0.0 value conversion:
  - Four string types (basic, multi-line basic, literal, multi-line literal)
  - Integers (decimal, hex, octal, binary, with underscore separators)
  - Floats (decimal, scientific, inf, nan, with underscore separators)
  - Booleans (true, false)
  - Date/time types (offset datetime, local datetime, local date, local time)
  - Arrays (homogeneous and heterogeneous)
  - Inline tables
- Semantic constraint enforcement:
  - Key uniqueness per table
  - Table path consistency
  - Inline table immutability
  - Array-of-tables consistency
