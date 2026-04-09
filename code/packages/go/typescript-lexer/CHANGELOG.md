# Changelog

## [0.2.0] - 2026-04-05

### Added
- `version string` parameter added to `TokenizeTypescript`, `CreateTypescriptLexer`,
  and the internal `getGrammarPath` helper.
- Valid versions: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`.
- Empty string `""` preserves the pre-0.2.0 behaviour (uses the generic
  `typescript.tokens` grammar), keeping existing callers backward-compatible.
- Unknown version strings return a descriptive error rather than silently
  falling back to the generic grammar.
- `required_capabilities.json` updated to declare all 7 allowed grammar file
  paths (1 generic + 6 versioned).
- `gen_capabilities.go` updated with 7 `_allowedPath_N` vars and updated
  `ReadFile` capability guard to match any of the 7 allowed paths.
- 7 new tests covering each versioned grammar and the error path for unknown
  versions.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go TypeScript lexer package.
- `TokenizeTypescript()` function that tokenizes TypeScript source code using the grammar-driven lexer.
- `CreateTypescriptLexer()` factory function.
- Loads `typescript.tokens` from `code/grammars/`.
- Supports TypeScript-specific keywords: `interface`, `type`, `number`, `string`, `boolean`, etc.
