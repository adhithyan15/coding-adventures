# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port of grammar-tools from Python and Go implementations.
- `parseTokenGrammar()` — parse `.tokens` files into structured `TokenGrammar` objects.
- `validateTokenGrammar()` — lint pass checking for duplicates, invalid regex, naming conventions.
- `tokenNames()` — extract the set of all defined token names.
- `parseParserGrammar()` — parse `.grammar` files (EBNF notation) into ASTs using a hand-written recursive descent parser.
- `validateParserGrammar()` — lint pass checking for undefined references, duplicates, unreachable rules.
- `ruleNames()`, `grammarTokenReferences()`, `grammarRuleReferences()` — AST query helpers.
- `crossValidate()` — check consistency between a token grammar and a parser grammar.
- TypeScript discriminated unions for grammar element types (`rule_reference`, `token_reference`, `literal`, `sequence`, `alternation`, `repetition`, `optional`, `group`).
- Full test suite ported from Python with vitest.
- Knuth-style literate programming comments preserved from the Python original.
