# Changelog

All notable changes to `coding_adventures_excel_lexer` will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::ExcelLexer.tokenize(source)` method that tokenizes JavaScript source code
- Loads `excel.tokens` grammar file and delegates to `GrammarLexer`
- Supports JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`
- Supports JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
- Full test suite with SimpleCov coverage >= 80%
