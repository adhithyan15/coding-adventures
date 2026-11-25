# Changelog

## [Unreleased]

### Security

- Fixed CodeQL `js/polynomial-redos` alert (`#254`) in `src/preprocessor.ts` by
  replacing the backtracking `` `define `` regex with a linear-time O(N)
  `parseDefineDirective` parser while preserving exact macro parameter/body
  semantics.
- Added ReDoS and macro-parsing regression tests in `tests/verilog-lexer.test.ts`.

## [0.1.0] - 2026-03-22

### Added

- Initial release of the TypeScript Verilog lexer
- Grammar-driven tokenization using `verilog.tokens`
- Built-in preprocessor for `define, `ifdef/`ifndef/`else/`endif, `include, `undef, `timescale
- Support for all Verilog token types (sized numbers, system IDs, directives, escaped identifiers, four-state operators, shift operators)
- Parameterized macro expansion with nested parenthesis support
- Conditional compilation
- 83 tests passing
