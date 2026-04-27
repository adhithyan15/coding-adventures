# Changelog — ECMAScript 1 (1997) Parser

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 1 (1997) parser
- Thin wrapper around `GrammarParser` loading `es1.grammar`
- Public API: `create_es1_parser(source)`, `parse_es1(source)`
- Comprehensive test suite verifying AST structure
