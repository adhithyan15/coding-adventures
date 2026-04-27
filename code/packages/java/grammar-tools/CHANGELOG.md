# Changelog

## 0.1.0 — 2026-04-04

### Added
- `TokenGrammarParser` — parses `.tokens` files into `TokenGrammar`
- `ParserGrammarParser` — parses `.grammar` files into `ParserGrammar`
- `TokenGrammarValidator` — semantic validation for token grammars
- `ParserGrammarValidator` — semantic validation for parser grammars
- `CrossValidator` �� cross-validation between token and parser grammars
- Support for magic comments (`# @version`, `# @case_insensitive`)
- Support for all sections: keywords, reserved, skip, errors, context_keywords, groups
- Support for all EBNF constructs: sequence, alternation, repetition, optional, group, lookaheads, separated repetition
- Full test suite with 40+ tests
