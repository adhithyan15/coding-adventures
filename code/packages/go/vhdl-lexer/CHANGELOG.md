# Changelog

All notable changes to the vhdl-lexer package will be documented in this file.

## [0.1.0] - 2026-03-22

### Added

- Initial implementation of the VHDL lexer for Go.
- `TokenizeVhdl()` — tokenize VHDL source with automatic case normalization.
- `CreateVhdlLexer()` — create a lexer instance directly (without case normalization).
- Case normalization: all NAME and KEYWORD token values are lowercased after tokenization.
- Support for all VHDL token types: character literals, bit strings, based literals, extended identifiers, keyword operators, string literals, and comments.
- Comprehensive test suite covering entity declarations, architectures, signal/variable/constant declarations, case insensitivity, all operator types, complete VHDL snippets, and edge cases.
