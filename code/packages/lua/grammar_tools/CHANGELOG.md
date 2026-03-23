# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Token grammar parser (`parse_token_grammar`) — parses `.tokens` files into
  `TokenGrammar` objects with definitions, keywords, reserved keywords, skip
  patterns, error recovery patterns, and named pattern groups
- Parser grammar parser (`parse_parser_grammar`) — parses `.grammar` files
  into `ParserGrammar` objects with EBNF-like production rules supporting
  sequences, alternation, repetition, optional elements, grouping, and
  string literals
- Token grammar validator (`validate_token_grammar`) — checks for duplicate
  names, empty patterns, naming convention violations, invalid modes/escapes,
  and empty pattern groups
- Parser grammar validator (`validate_parser_grammar`) — checks for duplicate
  rule names, non-lowercase names, undefined rule/token references, and
  unreachable rules
- Cross-validator (`cross_validate`) — checks consistency between token and
  parser grammars: missing token references (errors) and unused tokens
  (warnings)
- Support for `mode: indentation` and `escapes: none` directives
- Support for `-> ALIAS` syntax on token definitions
- Support for `group NAME:` sections for context-sensitive lexing
- Synthetic token support (NEWLINE, INDENT, DEDENT, EOF)
- `TokenGrammar:token_names()` and `TokenGrammar:effective_token_names()`
  helper methods
- `ParserGrammar:rule_names()`, `ParserGrammar:rule_references()`, and
  `ParserGrammar:token_references()` helper methods
- Comprehensive busted test suite (131 tests)
- Ported from Go implementation at `code/packages/go/grammar-tools/`
