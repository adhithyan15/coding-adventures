# Changelog

All notable changes to the Verilog Lexer package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the Verilog lexer package.
- `tokenize_verilog()` function that tokenizes Verilog HDL source code using the grammar-driven lexer.
- `create_verilog_lexer()` factory function with optional `preprocess` parameter.
- Verilog preprocessor (`verilog_preprocess`) as a `pre_tokenize` hook supporting:
  - `` `define `` — simple and parameterized macro definitions
  - `` `undef `` — macro removal
  - `` `ifdef / `ifndef / `else / `endif `` — conditional compilation with nesting
  - `` `include `` — stubbed with comment placeholder
  - `` `timescale `` — stripped from source
- Token support for Verilog-specific constructs:
  - Sized number literals: `4'b1010`, `8'hFF`, `32'd42`, with x/z values and underscores
  - System identifiers: `$display`, `$time`, `$finish`
  - Compiler directives: `` `define ``, `` `ifdef ``
  - Escaped identifiers: `\my.name`
  - Case equality/inequality: `===`, `!==`
  - Arithmetic shift: `<<<`, `>>>`
  - 46 synthesizable keywords
- Comprehensive test suite with 80%+ coverage.
