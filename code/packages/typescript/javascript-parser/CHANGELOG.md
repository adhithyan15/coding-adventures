# Changelog

All notable changes to the JavaScript Parser (TypeScript) package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `parseJavascript(source, version?)` — optional `version` parameter accepting
  `"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`.
  When omitted (or empty string), the generic grammars are used — backwards-compatible
  with v0.1.x.
- Versioned grammar support loads parser grammar from `code/grammars/ecmascript/<version>.grammar`
  and delegates to the versioned lexer grammar automatically.
- Clear error thrown for unrecognised version strings.
- Expanded test suite covering all supported ES version strings, empty-string version,
  and error cases.

### Changed
- `parseJavascript` signature is now `(source: string, version?: string): ASTNode`
  — fully backwards-compatible; existing callers with one argument are unaffected.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript JavaScript parser package.
- `parseJavascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- Loads `javascript.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/javascript-lexer`.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with v8 coverage.
