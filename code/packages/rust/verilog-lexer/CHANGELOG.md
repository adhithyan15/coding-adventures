# Changelog

All notable changes to the `coding-adventures-verilog-lexer` crate will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- `create_verilog_lexer(source)` — factory function that loads `verilog.tokens` and returns a configured `GrammarLexer`.
- `tokenize_verilog(source)` — convenience function that tokenizes Verilog source and returns `Vec<Token>`.
- `tokenize_verilog_preprocessed(source)` — tokenizes with preprocessing (macro expansion and conditional compilation).
- `preprocessor` module with `verilog_preprocess(source)` implementing:
  - `define/`undef — simple and parameterized macro expansion
  - `ifdef/`ifndef/`else/`endif — conditional compilation with nesting
  - `include — stubbed (comment placeholder)
  - `timescale — stripped
  - Line number preservation (directive lines replaced with empty lines)
- Loads grammar from `verilog.tokens` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering module declarations, keywords, arithmetic operators, multi-character operators (===, !==, <<<, >>>), strings, plain numbers, sized numbers (4'b1010, 8'hFF), real numbers, system identifiers ($display), directives, delimiters, comments, whitespace, sensitivity lists (@posedge), assign statements, preprocessed tokenization, conditional preprocessing, bitwise/logical/comparison/unary/ternary operators, full module example, escaped identifiers, power operator (**), and trigger operator (->).
- Preprocessor tests covering simple defines, parameterized macros, `undef, `ifdef/`ifndef/`else/`endif (including nesting), `include stub, `timescale stripping, line number preservation, multiple macros on one line, chained macro expansion, unknown macros, and defines in false branches.
