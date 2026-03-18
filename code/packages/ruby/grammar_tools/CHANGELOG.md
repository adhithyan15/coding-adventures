# Changelog

## [0.1.0] - 2026-03-18

### Added
- `TokenGrammar` class and `parse_token_grammar` for parsing `.tokens` files
- `TokenDefinition` data type with name, pattern, is_regex, and line_number
- `ParserGrammar` class and `parse_parser_grammar` for parsing `.grammar` files
- Full EBNF AST: `RuleReference`, `Literal`, `Sequence`, `Alternation`, `Repetition`, `OptionalElement`, `Group`
- `GrammarRule` data type with name, body, and line_number
- `validate_token_grammar` for linting token grammars
- `validate_parser_grammar` for linting parser grammars
- `cross_validate` for checking consistency between .tokens and .grammar files
- Reads real grammar files at `code/grammars/python.tokens` and `ruby.tokens`
