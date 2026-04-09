# Changelog

## [0.2.0] - 2026-04-05

### Added
- `version string` parameter added to `TokenizeJavascript`, `CreateJavascriptLexer`,
  and the internal `getGrammarPath` helper.
- Valid versions: `"es1"`, `"es3"`, `"es5"`, `"es2015"`, `"es2016"`, `"es2017"`,
  `"es2018"`, `"es2019"`, `"es2020"`, `"es2021"`, `"es2022"`, `"es2023"`,
  `"es2024"`, `"es2025"`.
- Empty string `""` preserves the pre-0.2.0 behaviour (uses the generic
  `javascript.tokens` grammar), keeping existing callers backward-compatible.
- Unknown version strings return a descriptive error rather than silently
  falling back to the generic grammar.
- `required_capabilities.json` updated to declare all 15 allowed grammar file
  paths (1 generic + 14 versioned).
- `gen_capabilities.go` updated with 15 `_allowedPath_N` vars and updated
  `ReadFile` capability guard to match any of the 15 allowed paths.
- 15 new tests covering each versioned grammar and the error path for unknown
  versions.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go JavaScript lexer package.
- `TokenizeJavascript()` function that tokenizes JavaScript source code using the grammar-driven lexer.
- `CreateJavascriptLexer()` factory function.
- Loads `javascript.tokens` from `code/grammars/`.
