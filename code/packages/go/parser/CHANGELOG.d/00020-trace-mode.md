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

