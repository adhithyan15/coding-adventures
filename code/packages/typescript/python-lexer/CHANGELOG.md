# Changelog

All notable changes to the Python Lexer (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenizePython` now imports a pre-compiled `_grammar[_<version>].ts` per Python version instead of `readFileSync`-ing the `.tokens` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`. Also added an explicit "Unknown Python version" error for invalid version strings — previously an invalid version fell through to a raw `readFileSync` `ENOENT`, which is a strictly worse error for callers.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Python lexer package.
- `tokenizePython()` function that tokenizes Python source code using the grammar-driven lexer.
- Loads `python.tokens` grammar file from `code/grammars/`.
- Supports Python keywords: `if`, `else`, `elif`, `while`, `for`, `def`, `return`, `class`, `import`, `from`, `as`, `True`, `False`, `None`.
- Supports operators: `+`, `-`, `*`, `/`, `=`, `==`.
- Supports delimiters: `(`, `)`, `,`, `:`.
- Supports string literals, numeric literals, and identifiers.
- Comprehensive test suite with v8 coverage.
