# Changelog

All notable changes to `coding_adventures_verilog_lexer` will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release
- `CodingAdventures::VerilogLexer.tokenize(source, preprocess:)` method that tokenizes Verilog HDL source code
- Loads `verilog.tokens` grammar file and delegates to `GrammarLexer`
- `CodingAdventures::VerilogLexer::Preprocessor.process(source)` for resolving compiler directives
- Preprocessor supports `define/`undef for simple and parameterized macros
- Preprocessor supports `ifdef/`ifndef/`else/`endif conditional compilation
- Preprocessor stubs `include directives (emits comment)
- Preprocessor strips `timescale directives
- `preprocess:` keyword option on `tokenize` to enable preprocessing before lexing
- Supports Verilog keywords: module, endmodule, wire, reg, assign, always, begin, end, if, else, case, etc.
- Supports sized numbers: 4'b1010, 8'hFF, 32'd42
- Supports system tasks: $display, $time, $finish
- Supports escaped identifiers: \my_name
- Supports three-character operators: <<<, >>>, ===, !==
- Full test suite with SimpleCov coverage >= 80%
