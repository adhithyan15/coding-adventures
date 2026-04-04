# Changelog

## [0.5.0] - 2026-04-04

### Added
- **TokenGrammar**: `context_keywords` attribute -- list of context-sensitive
  keywords parsed from the `context_keywords:` section in `.tokens` files
- **Token grammar parser**: recognises `context_keywords:` section header and
  collects indented keyword entries (same format as `keywords:` section)
- **ParserGrammar element types**:
  - `PositiveLookahead` -- `&element` predicate (match without consuming)
  - `NegativeLookahead` -- `!element` predicate (succeed if element fails)
  - `OneOrMoreRepetition` -- `{ element }+` (one-or-more)
  - `SeparatedRepetition` -- `{ element // separator }` with optional `+` suffix
- **Grammar tokenizer**: recognises `&`, `!`, `+`, and `//` tokens in `.grammar` files
- **EBNF parser**: `parse_element` handles all four new element types
- **Collect helpers**: `collect_token_refs` and `collect_rule_refs` traverse new
  element types correctly
- **Compiler**: `element_src` generates Ruby constructor calls for all four new
  element types; `compile_token_grammar` includes `context_keywords` in output

## [0.4.0] - 2026-03-26

### Added
- `Compiler` module (`lib/coding_adventures/grammar_tools/compiler.rb`) with:
  - `compile_token_grammar(grammar, source_file = "") → String` — generates Ruby source
    embedding a `TokenGrammar` as native `GT::TokenGrammar.new(...)` call.
  - `compile_parser_grammar(grammar, source_file = "") → String` — generates Ruby source
    embedding a `ParserGrammar` as native `GT::ParserGrammar.new(...)` call.
  - All grammar element types supported: `RuleReference`, `Literal`, `Sequence`,
    `Alternation`, `Repetition`, `OptionalElement`, `Group`.
- Module-level convenience wrappers on `GrammarTools`:
  `GrammarTools.compile_token_grammar(...)` and `GrammarTools.compile_parser_grammar(...)`.
- `test/test_compiler.rb` — 30 tests covering output structure, round-trip fidelity for
  all grammar features: aliases, skip/error defs, groups, keywords, mode, escape_mode,
  case_insensitive, version, special regex chars, full JSON grammar round-trip.

## [0.3.0] - 2026-03-23

### Added
- `bin/grammar-tools` CLI executable with three subcommands:
  - `validate <file.tokens> <file.grammar>` — validates both files and cross-validates them
  - `validate-tokens <file.tokens>` — validates a .tokens file in isolation
  - `validate-grammar <file.grammar>` — validates a .grammar file in isolation
  - `--help` / `-h` / `help` — prints usage information
- Output format matches the Python grammar-tools CLI exactly (e.g., `OK (N tokens, M skip)`)
- Exit codes: 0 = pass, 1 = validation errors, 2 = usage error
- gemspec updated: `spec.executables = ["grammar-tools"]` and `spec.bindir = "bin"`
- `spec.files` updated to include `bin/*`
- Test suite `test/test_cli.rb` with 23 tests covering all subcommands, exit codes,
  output format, missing files, and wrong argument counts

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
