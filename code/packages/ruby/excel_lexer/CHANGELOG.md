# Changelog

All notable changes to `coding_adventures_excel_lexer` will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `create_excel_lexer` now loads the pre-compiled `_grammar.rb` (via `GrammarTools::CompiledLoader`) instead of reading and parsing `excel.tokens` from `code/grammars/` on every call. The old code walked out of the installed gem's own directory to a monorepo-relative path that a published gem does not ship, so a real `gem install` + first use would raise `Errno::ENOENT`.

## [0.1.0] - 2026-03-19

### Added
- Initial release
- `CodingAdventures::ExcelLexer.tokenize(source)` method that tokenizes JavaScript source code
- Loads `excel.tokens` grammar file and delegates to `GrammarLexer`
- Supports JavaScript keywords: `let`, `const`, `var`, `function`, `if`, `else`, `while`, `for`, `return`, `class`, `true`, `false`, `null`, `undefined`
- Supports JavaScript-specific operators: `===`, `!==`, `=>`, `==`, `!=`, `<=`, `>=`
- Supports delimiters: `()`, `{}`, `[]`, `;`, `,`, `:`, `.`
- Full test suite with SimpleCov coverage >= 80%
