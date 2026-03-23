# Changelog

## 0.1.0 — 2026-03-22

### Added
- `VerilogLexer.tokenize/2` — tokenize Verilog source code with optional preprocessing
- `VerilogLexer.create_lexer/0` — parse verilog.tokens grammar
- `VerilogLexer.Preprocessor.process/1` — resolve Verilog compiler directives
  - `define/`undef with simple and parameterized macro expansion
  - `ifdef/`ifndef/`else/`endif conditional compilation with nesting
  - `include file inclusion (stubbed with comment placeholder)
  - `timescale stripping
- Grammar caching via `persistent_term` for repeated use
- 55+ tests covering tokenization, preprocessing, and full pipeline integration
