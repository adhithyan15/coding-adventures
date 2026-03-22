# Changelog

## [0.2.0] - 2026-03-21

### Added
- `PatternGroup` data type (Data.define) with `name` and `definitions` fields
- `groups` hash field on `TokenGrammar` for named pattern groups
- `group NAME:` section parsing in `parse_token_grammar` with validation:
  - Group name must match `[a-z_][a-z0-9_]*`
  - Reserved names (default, skip, keywords, reserved, errors) are rejected
  - Duplicate group names are rejected
  - Group definitions use the same definition parser as other sections
- `token_names` and `effective_token_names` updated to include group definitions
- Group validation in `validate_token_grammar`: bad regex, empty groups, naming conventions
- Comprehensive test suite for pattern group parsing, validation, and error handling

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
