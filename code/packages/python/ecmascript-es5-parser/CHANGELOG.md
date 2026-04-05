# Changelog — ECMAScript 5 (2009) Parser

## 0.1.0 (2026-04-04)

### Added

- Initial implementation of the ECMAScript 5 (2009) parser
- Thin wrapper around `GrammarParser` loading `es5.grammar`
- Public API: `create_es5_parser(source)`, `parse_es5(source)`
- Comprehensive test suite verifying AST structure
