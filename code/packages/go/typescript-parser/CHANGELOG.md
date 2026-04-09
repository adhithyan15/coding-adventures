# Changelog

## [0.2.0] - 2026-04-05

### Added
- `version string` parameter added to `ParseTypescript`, `CreateTypescriptParser`,
  and the internal `getGrammarPath` helper.
- Valid versions: `"ts1.0"`, `"ts2.0"`, `"ts3.0"`, `"ts4.0"`, `"ts5.0"`, `"ts5.8"`.
- Empty string `""` preserves the pre-0.2.0 behaviour (uses the generic
  `typescript.grammar`), keeping existing callers backward-compatible.
- Version string is forwarded to the underlying `typescript-lexer` call so
  lexer and parser grammars are always selected consistently.
- Unknown version strings return a descriptive error rather than silently
  routing to the wrong grammar.
- `required_capabilities.json` updated to declare all 7 allowed grammar file
  paths (1 generic + 6 versioned).
- `gen_capabilities.go` updated with 7 `_allowedPath_N` vars and updated
  `ReadFile` capability guard to match any of the 7 allowed paths.
- 7 new tests covering each versioned grammar and the error path for unknown
  versions.

## [0.1.1] - 2026-03-31

### Fixed

- Fixed parse failures for `let`/`const` variable declarations by relying on
  the shared fix to `typescript.grammar`. The grammar now uses `KEYWORD` instead
  of `VAR | LET | CONST`, matching what the lexer actually emits.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go TypeScript parser package.
- `ParseTypescript()` function that parses TypeScript source code into generic `ASTNode` trees.
- `CreateTypescriptParser()` factory function.
- Loads `typescript.grammar` from `code/grammars/`.
