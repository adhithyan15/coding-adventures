# Changelog

All notable changes to the VHDL Lexer package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release of the VHDL lexer package.
- `tokenize_vhdl()` function that tokenizes VHDL source code using the grammar-driven lexer with case normalization.
- `create_vhdl_lexer()` factory function for raw tokenization without normalization.
- Case normalization post-processing: NAME and KEYWORD token values are lowercased to implement VHDL's case insensitivity.
- Token support for VHDL-specific constructs:
  - Character literals: `'0'`, `'1'`, `'X'`, `'Z'`
  - Bit string literals: `B"1010"`, `X"FF"`, `O"77"`, `D"42"`
  - Based literals: `16#FF#`, `2#1010#`, `8#77#`
  - Real numbers: `3.14`, `1.5e3`, `2.0E-3`
  - Extended identifiers: `\my name\` (case-preserving)
  - Doubled-quote string escaping: `"He said ""hello"""`
  - Two-character operators: `:=`, `<=`, `>=`, `=>`, `/=`, `**`, `<>`
  - 97 keywords including operator keywords: `and`, `or`, `xor`, `not`, `mod`, `rem`, `sll`, `srl`
- Comprehensive test suite with 95%+ coverage.
