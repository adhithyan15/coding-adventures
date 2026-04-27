# Changelog — ECMAScript 3 (1999) Parser

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 3 (1999) parser
- Thin wrapper around `GrammarParser` loading `es3.grammar`
- Public API: `create_es3_parser(source)`, `parse_es3(source)`
- Comprehensive test suite verifying AST structure
