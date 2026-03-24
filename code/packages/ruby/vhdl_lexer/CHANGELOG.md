# Changelog

All notable changes to `coding_adventures_vhdl_lexer` will be documented in this file.

## [0.1.0] - 2026-03-22

### Added
- Initial release
- `CodingAdventures::VhdlLexer.tokenize(source)` method that tokenizes VHDL source code
- Loads `vhdl.tokens` grammar file and delegates to `GrammarLexer`
- Post-tokenization case normalization: NAME and KEYWORD token values are lowercased
- Supports VHDL keywords: entity, architecture, signal, port, process, if, then, else, etc.
- Supports based literals: 16#FF#, 2#1010#
- Supports bit string literals: X"FF", B"1010", O"77"
- Supports character literals: '0', '1', 'Z', 'X'
- Supports real numbers: 3.14, 1.5e-3
- Supports extended identifiers: \my_name\
- Supports two-character operators: :=, <=, >=, =>, /=, **, <>
- Full test suite with SimpleCov coverage >= 80%
