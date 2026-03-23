# Changelog

All notable changes to this package will be documented in this file.

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
