# Changelog — TypeScript 3.0 (2018) Parser

## 0.1.1 (2026-04-05)

### Fixed

- Updated `function_declaration`, `async_function_declaration`,
  `generator_declaration`, `async_generator_declaration`, and
  `function_expression` in ts3.0.grammar to use `typed_parameter_list`
  instead of `formal_parameters`, enabling TypeScript type annotations
  on parameters (`function foo(x: string): void {}`)
- Added `[ type_parameters ]` and `[ COLON type_expression ]` to all function
  rules for generics and return type annotations
- Fixed `variable_declaration` and `lexical_binding` to include
  `[ COLON type_expression ]` for typed variable declarations
- Fixed `arrow_parameters` to include return type annotation and generics
- Fixed `type_member` ordering: `construct_signature` before `property_signature`
- Added `ambient_function_declaration` rule to ts3.0.grammar

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 3.0 (2018) parser
- Thin wrapper around `GrammarParser` loading `ts3.0.grammar`
- Public API: `create_ts30_parser(source)`, `parse_ts30(source)`
- Comprehensive test suite verifying AST structure for TS 3.0 features
- Tests for `unknown` type, type annotations, interfaces, classes, control flow
