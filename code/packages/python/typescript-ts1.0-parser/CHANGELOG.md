# Changelog — TypeScript 1.0 (April 2014) Parser

## 0.1.1 (2026-04-05)

### Fixed

- Fixed `type_member` ordering in ts1.0.grammar: `construct_signature` and
  `call_signature` are now tried before `method_signature` and
  `property_signature` to prevent greedy matching of method names as properties
- Added `ambient_function_declaration` rule (bodyless function) to ts1.0.grammar
  so `declare function foo(x: string): void;` parses correctly

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 1.0 (April 2014) parser
- Thin wrapper around `GrammarParser` loading `ts1.0.grammar`
- Public API: `create_ts10_parser(source)`, `parse_ts10(source)`
- Comprehensive test suite covering interface declarations, type aliases,
  enum declarations, ambient declarations, class declarations, namespace
  declarations, typed function declarations, and ES5 compatibility
- PEP 561 `py.typed` marker for type checker support
