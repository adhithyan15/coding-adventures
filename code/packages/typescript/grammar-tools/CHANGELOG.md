# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-03-23

### Added

- `src/cli.ts` — CLI entry point for grammar validation. Implements three subcommands:
  - `grammar-tools validate <file.tokens> <file.grammar>` — validates both files individually and cross-validates them.
  - `grammar-tools validate-tokens <file.tokens>` — validates just a `.tokens` file.
  - `grammar-tools validate-grammar <file.grammar>` — validates just a `.grammar` file.
  - `grammar-tools --help` / `-h` / `help` — prints usage information.
- Output format matches the Python `grammar_tools` CLI: `OK (N tokens, M skip)`, `OK (P rules)`, `Cross-validating ... OK`, `All checks passed.` / `Found N error(s). Fix them and try again.`
- Exit codes: 0 = all checks passed, 1 = validation errors, 2 = usage error.
- `"bin": { "grammar-tools": "./dist/cli.js" }` added to `package.json` so the CLI binary is installed when the package is installed globally or as a local dep.
- 29 new tests in `tests/cli.test.ts` covering all subcommands, error paths, exit codes, usage output, and `main()` dispatch. Uses in-process function calls rather than subprocess spawning for speed and reliability.

## [0.2.0] - 2026-03-21

### Added

- `PatternGroup` interface for named sets of token definitions that enable context-sensitive lexing.
- `groups` optional field on `TokenGrammar` interface — a record of named pattern groups.
- `group NAME:` section parsing in `parseTokenGrammar()` with full validation:
  - Group names must be lowercase identifiers matching `[a-z_][a-z0-9_]*`.
  - Reserved names (`default`, `skip`, `keywords`, `reserved`, `errors`) are rejected.
  - Duplicate group names are rejected.
  - Group definitions use the same definition parser as other sections (regex, literal, aliases).
- `effectiveTokenNames()` function — returns token names as the parser will see them (aliases replace original names).
- `tokenNames()` now includes names from all pattern groups.
- Group validation in `validateTokenGrammar()`: bad regex detection, empty group warnings, naming convention checks.
- 20 new test cases covering pattern group parsing, validation, and error handling.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port of grammar-tools from Python and Go implementations.
- `parseTokenGrammar()` — parse `.tokens` files into structured `TokenGrammar` objects.
- `validateTokenGrammar()` — lint pass checking for duplicates, invalid regex, naming conventions.
- `tokenNames()` — extract the set of all defined token names.
- `parseParserGrammar()` — parse `.grammar` files (EBNF notation) into ASTs using a hand-written recursive descent parser.
- `validateParserGrammar()` — lint pass checking for undefined references, duplicates, unreachable rules.
- `ruleNames()`, `grammarTokenReferences()`, `grammarRuleReferences()` — AST query helpers.
- `crossValidate()` — check consistency between a token grammar and a parser grammar.
- TypeScript discriminated unions for grammar element types (`rule_reference`, `token_reference`, `literal`, `sequence`, `alternation`, `repetition`, `optional`, `group`).
- Full test suite ported from Python with vitest.
- Knuth-style literate programming comments preserved from the Python original.
