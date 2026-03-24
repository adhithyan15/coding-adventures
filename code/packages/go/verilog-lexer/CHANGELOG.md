# Changelog

All notable changes to the verilog-lexer package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Initial implementation of the Verilog lexer for Go.
- `TokenizeVerilog()` — tokenize Verilog source with preprocessing enabled.
- `TokenizeVerilogRaw()` — tokenize without preprocessing (directives appear as tokens).
- `CreateVerilogLexer()` / `CreateVerilogLexerRaw()` — create lexer instances directly.
- `VerilogPreprocess()` — standalone preprocessor for Verilog source text.
- `VerilogPreprocessWithDefines()` — preprocessor with predefined macro support.
- Preprocessor supports: `define (simple and parameterized), `undef, `ifdef/`ifndef/`else/`endif, `include (stubbed), `timescale (stripped).
- Nested conditional compilation with correct condition stack management.
- Line number preservation (empty lines for excluded/stripped directives).
- Parameterized macro expansion with nested parenthesis handling.
- Comprehensive test suite for both lexer and preprocessor (80%+ coverage target).
