# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-04

### Added
- `TokenGrammar` struct and `parseTokenGrammar(source:)` function for parsing `.tokens` files
- `TokenDefinition`, `PatternGroup` data types for token grammar model
- Support for keywords, reserved keywords, skip patterns, and pattern groups
- **New**: `contextKeywords` field and `context_keywords:` section parsing for context-sensitive keywords
- Magic comment support (`# @version`, `# @case_insensitive`)
- `validateTokenGrammar()` lint pass for common issues
- `tokenNames()` and `effectiveTokenNames()` helper functions
- `GrammarElement` enum with all standard EBNF variants plus extensions:
  - `.positiveLookahead` -- `& element` syntax
  - `.negativeLookahead` -- `! element` syntax
  - `.oneOrMore` -- `element +` syntax
  - `.separatedRepetition` -- `element // separator` syntax
- `ParserGrammar` struct and `parseParserGrammar(source:)` function for parsing `.grammar` files
- `GrammarRule` data type for parser grammar model
- `validateParserGrammar()` lint pass for common issues
- `tokenReferences()` and `ruleReferences()` helper functions
- `crossValidate()` function for checking `.tokens`/`.grammar` consistency
- Comprehensive test suite with 40+ test cases
