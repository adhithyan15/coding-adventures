# Changelog

All notable changes to the Ruby Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Ruby parser package.
- `parseRuby()` function that parses Ruby source code into generic `ASTNode` trees.
- Loads `ruby.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/ruby-lexer`.
- Supports assignments, method calls, arithmetic expressions, and operator precedence.
- Comprehensive test suite with v8 coverage.
