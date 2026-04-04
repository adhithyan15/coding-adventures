# Changelog

## [0.5.0] - 2026-04-04

### Added
- **Context keywords section**: `ContextKeywords []string` field on `TokenGrammar`,
  populated by the new `context_keywords:` section in `.tokens` files. Context
  keywords are words that are keywords in some syntactic positions but identifiers
  in others (e.g., JavaScript's `async`, `yield`, `get`, `set`).
- **Positive lookahead** (`PositiveLookahead` struct): `&element` syntax in `.grammar`
  files. Succeeds without consuming input if element matches at the current position.
- **Negative lookahead** (`NegativeLookahead` struct): `!element` syntax. Succeeds
  without consuming input if element does NOT match.
- **One-or-more repetition** (`OneOrMoreRepetition` struct): `{ element }+` syntax.
  Like zero-or-more but requires at least one match.
- **Separated repetition** (`SeparatedRepetition` struct): `{ element // separator }`
  syntax (with optional `+` suffix for one-or-more). Matches element occurrences
  separated by separator.
- Tokenizer now recognizes `&`, `!`, `+`, and `//` as grammar tokens.
- `collectRuleRefs` and `collectTokenRefs` handle all four new element types.
- Compiler (`goElementLit`) generates Go literals for all four new element types.

## [0.4.0] - 2026-03-26

### Added
- `compiler.go` — `CompileTokenGrammar(grammar *TokenGrammar, sourceFile, pkgName string) string`
  and `CompileParserGrammar(grammar *ParserGrammar, sourceFile, pkgName string) string`.
  Both functions generate Go source code that embeds the grammar as native Go data structures.
  Uses raw string literals (backtick) for most patterns; falls back to double-quoted strings
  when a pattern contains a backtick.
- 34 new tests in `compiler_test.go` covering header structure, field encoding, all grammar
  element types, and edge cases.

## [0.3.0] - Unreleased

### Added

- `ErrorDefinitions []TokenDefinition` field on `TokenGrammar` (between `SkipDefinitions` and `ReservedKeywords`).
  Populated by the new `errors:` section in `.tokens` files — patterns tried as a fallback when no normal token matches (e.g., `BAD_STRING` for unclosed strings in CSS).
- Parsing logic for the `errors:` section in `ParseTokenGrammar` (mirrors `skip:` parsing).
- Validation of `ErrorDefinitions` in `ValidateTokenGrammar` (calls `validateDefinitions(grammar.ErrorDefinitions, "error pattern")`).
- `ValidateParserGrammar(grammar *ParserGrammar, tokenNames map[string]bool) []string` — lint pass for parser grammars checking: duplicate rule names, non-lowercase rule names, undefined rule references, undefined token references (when `tokenNames` provided), and unreachable rules.
- `RuleNames() map[string]bool` method on `*ParserGrammar` — returns the set of all defined rule names.
- `RuleReferences() map[string]bool` method on `*ParserGrammar` — returns all lowercase rule names referenced in rule bodies.
- `TokenReferences() map[string]bool` method on `*ParserGrammar` — returns all UPPERCASE names referenced in rule bodies.
- `cross_validator.go` — new file with `CrossValidate(tokenGrammar *TokenGrammar, parserGrammar *ParserGrammar) []string`. Checks that every UPPERCASE name in the grammar is defined in the token grammar (error), and warns about tokens defined but never used. Synthetic tokens (NEWLINE, INDENT, DEDENT, EOF) are always valid.
- `cmd/grammar-tools/main.go` — CLI binary with three subcommands: `validate <file.tokens> <file.grammar>`, `validate-tokens <file.tokens>`, `validate-grammar <file.grammar>`. Uses `os.Args` directly (no external CLI library). Exit codes: 0 = pass, 1 = errors found, 2 = usage error.
- Comprehensive test suite for all new functionality: `ErrorDefinitions` parsing and validation (6 tests), `ValidateParserGrammar` (8 tests), `RuleNames`/`RuleReferences`/`TokenReferences` (3 tests), `CrossValidate` (4 tests).

## [0.2.0] - Unreleased

### Added
- `PatternGroup` struct for named sets of token definitions used in context-sensitive lexing.
- `Groups` field on `TokenGrammar` (`map[string]*PatternGroup`) populated by `group NAME:` sections.
- `group NAME:` section parsing with validation: lowercase identifier names, reserved name rejection (`default`, `skip`, `keywords`, `reserved`, `errors`), duplicate name rejection, and standard definition parsing within groups.
- `EffectiveTokenNames()` method on `TokenGrammar` returning alias-resolved token names (includes group definitions).
- `ValidateTokenGrammar()` function performing lint-style checks: duplicate names, invalid regexes, empty patterns, naming conventions, mode/escape validation, empty group warnings, and group definition validation.
- `TokenNames()` now includes token names from all pattern groups.
- Comprehensive test suite for pattern group parsing, validation, and error handling (13 new tests).

## [0.1.0]

### Added
- Parser resolving explicit `.tokens` structures handling language-agnostic boundary definitions parsing effectively bypassing logic overlaps iteratively safely securely translating boundaries efficiently executing natively without cross-language imports natively.
