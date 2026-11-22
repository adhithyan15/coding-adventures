# Changelog

## 0.1.0

- Initial release of `coding_adventures_verilog_lexer`.
- Pure Dart grammar-driven lexical analyzer and preprocessor for Verilog HDL.
- Support for IEEE 1364-1995, IEEE 1364-2001, and IEEE 1364-2005 editions.
- In-memory line-preserving preprocessor handling `define, `undef, `ifdef, `ifndef, `else, `endif, and parameterized macros.
- Defense against macro expansion bombs (CWE-400) and surrogate validation.
- 100% compliance with Standing Rule #20 (centralized diagnostics, sealed exception hierarchy, payload blindness).
