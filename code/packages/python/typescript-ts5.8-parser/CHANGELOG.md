# Changelog — TypeScript 5.8 (2025) Parser

## 0.1.2 (2026-04-05)

- Add `BUILD_windows` for Windows CI compatibility (unquoted `.[dev]`, `uv run` instead of `.venv/bin/python`)
- Fix `read_text()` to use `encoding="utf-8"` explicitly (prevents `UnicodeDecodeError` on Windows cp1252)

## 0.1.1 (2026-04-05)

### Fixed

- Updated all function rules to use `typed_parameter_list` instead of
  `formal_parameters` in ts5.8.grammar
- Fixed `variable_declaration` and `lexical_binding` to include
  `[ COLON type_expression ]` for typed variable declarations
- Fixed `arrow_parameters` to support return type annotations and generics
- Fixed `type_member` ordering in ts5.8.grammar
- Added `ambient_function_declaration` rule to ts5.8.grammar

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 5.8 (2025) parser
- Thin wrapper around `GrammarParser` loading `ts5.8.grammar`
- Public API: `create_ts58_parser(source)`, `parse_ts58(source)`
- Comprehensive test suite covering variable statements, `using`/`await using`
  resource management declarations, interface declarations, type alias
  declarations, export type statements (including `export type *`), ambient
  module declarations, class declarations with decorators, generic functions,
  enum declarations, and multi-declaration programs