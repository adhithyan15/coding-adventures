# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `token_grammar` module: parse `.tokens` files into `TokenGrammar` structs
  - `TokenDefinition` struct with name, pattern, is_regex, and line_number
  - `TokenGrammar` struct with definitions and keywords
  - `parse_token_grammar()` function with detailed error messages
  - `validate_token_grammar()` lint pass for duplicates, invalid regex, naming conventions
  - `token_names()` helper to extract defined token names
- `parser_grammar` module: parse `.grammar` files (EBNF) into `ParserGrammar` structs
  - `GrammarElement` enum with 8 variants: RuleReference, TokenReference, Literal, Sequence, Alternation, Repetition, Optional, Group
  - `GrammarRule` and `ParserGrammar` structs
  - Hand-written recursive descent parser (tokenizer + parser)
  - `validate_parser_grammar()` for undefined references, duplicates, unreachable rules
  - `rule_names()`, `grammar_token_references()`, `grammar_rule_references()` helpers
- `cross_validator` module: check consistency between `.tokens` and `.grammar` files
  - Reports missing token references as errors
  - Reports unused tokens as warnings
- Comprehensive test suite covering parsing, validation, error cases, and cross-validation
