# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- `BUILD_windows`: `where swift` now discards its output with cmd's `>nul
  2>nul` instead of `>/dev/null`. Under `cmd /C` the `/dev/null` redirect
  always failed, so the line always fell through to its "Swift not available"
  echo and the Swift step never ran on Windows, even on runners with Swift
  installed.

## [0.2.0] - 2026-07-13

### Added
- `GrammarCompiler.swift`: `compileTokenGrammarExpression(_:)` and
  `compileParserGrammarExpression(_:)` render a parsed grammar back into Swift
  source that reconstructs it as native `TokenGrammar` / `ParserGrammar` values
  (plus `swiftStringLiteral(_:)`, an exhaustive Swift string-literal escaper).
- `grammar-tools-embed` executable target: reads canonical grammar file(s),
  parses them with GrammarTools' own parser, and writes a package's
  `_Grammar.swift` embedding the grammar(s). This lets lexer/parser packages
  compile the grammar in instead of reading `code/grammars/**` at run time.

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
