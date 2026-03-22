# Changelog

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
