# Changelog

All notable changes to this package will be documented in this file.

## [0.3.0] - 2026-04-04

### Added

- `context_keywords` field on TokenGrammar for context-sensitive keywords
- `context_keywords:` section parsing in `parse_token_grammar`
- Four new parser grammar element types:
  - `positive_lookahead` (`&element`) — succeeds if element matches, consumes nothing
  - `negative_lookahead` (`!element`) — succeeds if element does NOT match, consumes nothing
  - `one_or_more` (`{ element }+`) — one-or-more repetition
  - `separated_repetition` (`{ element // separator }`) — separated list with optional `+` suffix
- Grammar tokenizer handles `&`, `!`, `+`, `//` tokens
- Grammar parser handles all four new element types
- Element constructors: `make_positive_lookahead`, `make_negative_lookahead`, `make_one_or_more`, `make_separated_repetition`
- Updated `collect_rule_refs` and `collect_token_refs` to walk new element types

## [0.2.0] - 2026-03-26

### Added
- `compiler.lua` — new `Compiler` sub-module with:
  - `compile_token_grammar(grammar, source_file)` — generates Lua source code
    embedding a `TokenGrammar` as native Lua data structures. Generated file
    returns `{ token_grammar = token_grammar }` where `token_grammar()` returns
    a fully-populated `TokenGrammar` instance without file I/O.
  - `compile_parser_grammar(grammar, source_file)` — generates Lua source code
    embedding a `ParserGrammar`. Handles all seven element types: `rule_reference`,
    `literal`, `sequence`, `alternation`, `repetition`, `optional`, `group`.
  - Uses `string.format("%q", s)` throughout for correct Lua string escaping.
  - Group keys are sorted for deterministic output.
- Delegations on the top-level `grammar_tools` module:
  `compile_token_grammar` and `compile_parser_grammar`.
- `tests/test_compiler.lua` — 25+ tests covering output structure and full
  round-trip fidelity for all grammar features.

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
