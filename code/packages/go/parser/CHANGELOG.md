# Changelog

## [0.2.0] - Unreleased

### Added

- `trace bool` field on `GrammarParser` struct.
- `NewGrammarParserWithTrace(tokens []Token, grammar *ParserGrammar, trace bool) *GrammarParser` constructor.
  When `trace=true`, each rule attempt is printed to stderr:
  `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match`
  `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → fail`
  Packrat memoization means each `(rule, position)` pair is traced at most once.
- `NewGrammarParser` now delegates to `NewGrammarParserWithTrace(..., false)`.
- Trace mode tests: `TestGrammarParserWithTraceNoError`, `TestGrammarParserWithTraceMatchesNoTrace`, `TestGrammarParserWithTraceFailurePath`.

## [0.1.0] - Unreleased

### Added
- Parser LL(2) structs navigating `Tokens` natively into explicit AST structs mappings. 
- Integrated native Parser structure extracting syntax bindings recursively resolving mathematical orders without generic limits.
