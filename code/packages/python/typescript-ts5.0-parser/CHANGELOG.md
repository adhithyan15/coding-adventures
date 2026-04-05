# Changelog — TypeScript 5.0 (2023) Parser

## 0.1.1 (2026-04-05)

### Fixed

- Updated all function rules to use `typed_parameter_list` instead of
  `formal_parameters` in ts5.0.grammar
- Fixed `variable_declaration` and `lexical_binding` to include
  `[ COLON type_expression ]` for typed variable declarations
- Fixed `arrow_parameters` to support return type annotations and generics
- Fixed `type_member` ordering in ts5.0.grammar
- Added `ambient_function_declaration` rule to ts5.0.grammar

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 5.0 (2023) parser
- Thin wrapper around `GrammarParser` loading `ts5.0.grammar`
- Public API: `create_ts50_parser(source)`, `parse_ts50(source)`
- Comprehensive test suite covering variable statements, interface declarations,
  type alias declarations, enum declarations, class declarations with decorators,
  generic functions with type parameters, and multiple top-level declarations
