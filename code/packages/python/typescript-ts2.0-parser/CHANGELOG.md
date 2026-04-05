# Changelog — TypeScript 2.0 (September 2016) Parser

## 0.1.1 (2026-04-05)

### Fixed

- Fixed `arrow_parameters` in ts2.0.grammar: added `[ COLON type_expression ]`
  after `LPAREN [ typed_parameter_list ] RPAREN` to support return type
  annotations on arrow functions: `(x: string): string => x`
- Moved `enum` from `reserved:` to `keywords:` in ts2.0.tokens so enum
  declarations parse correctly instead of raising `LexerError`
- Added `ambient_function_declaration` rule to ts2.0.grammar

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 2.0 (September 2016) parser
- Thin wrapper around `GrammarParser` loading `ts2.0.grammar`
- Public API: `create_ts20_parser(source)`, `parse_ts20(source)`
- Comprehensive test suite covering arrow functions, ES2015 classes,
  ES2015 modules (import/export), the `never` type, interface declarations,
  type aliases, destructuring, TS 1.0 compatibility, and ES5 compatibility
- PEP 561 `py.typed` marker for type checker support
