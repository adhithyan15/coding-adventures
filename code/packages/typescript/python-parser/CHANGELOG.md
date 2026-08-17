# Changelog

All notable changes to the Python Parser (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parsePython` now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `python.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`. (Unlike `python-lexer`, this parser is not version-selectable — it always parses against the single generic grammar.)

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Python parser package.
- `parsePython()` function that parses Python source code into generic `ASTNode` trees.
- Loads `python.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/python-lexer`.
- Supports assignments, arithmetic expressions, operator precedence, and multiple statements.
- Comprehensive test suite with v8 coverage.
