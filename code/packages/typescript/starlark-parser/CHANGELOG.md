# Changelog

All notable changes to the Starlark Parser (TypeScript) package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Starlark parser package.
- `parseStarlark()` function that parses Starlark source code into generic `ASTNode` trees.
- Loads `starlark.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/starlark-lexer`.
- Supports assignments (simple and augmented), function definitions, if/elif/else, for loops, load statements, BUILD-file style function calls with named arguments, list/dict literals, comprehensions, and full operator precedence.
- Comprehensive test suite with v8 coverage.
