# Changelog

## [0.1.0] - 2026-03-22

### Added

- Initial release of the TypeScript VHDL lexer
- Grammar-driven tokenization using `vhdl.tokens`
- Case normalization: NAME and KEYWORD values lowercased to reflect VHDL's case insensitivity
- Support for all VHDL token types (character literals, bit strings, based literals, extended identifiers, keyword operators)
- Tests covering: entity declarations, architecture blocks, case insensitivity, character literals, bit strings, operators, keywords, comments, complete design units
