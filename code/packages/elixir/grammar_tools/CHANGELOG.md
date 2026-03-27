# Changelog

## [0.4.0] - 2026-03-26

### Added
- `CodingAdventures.GrammarTools.Compiler` module (`lib/grammar_tools/compiler.ex`) with:
  - `compile_token_grammar/2` — generates Elixir source with `def token_grammar/0 → %TokenGrammar{}`.
  - `compile_parser_grammar/2` — generates Elixir source with `def parser_grammar/0 → %ParserGrammar{}`.
  - Supports all grammar element tagged tuple types: `:rule_reference`, `:literal`, `:sequence`,
    `:alternation`, `:repetition`, `:optional`, `:group`.
  - Uses Elixir's `inspect/1` for string literals — correct quoting and escaping with zero
    manual logic.
- Convenience delegations on `CodingAdventures.GrammarTools`:
  `compile_token_grammar/1,2` and `compile_parser_grammar/1,2`.
- `test/grammar_tools/compiler_test.exs` — 30 tests covering output structure, round-trip
  fidelity for all grammar features, and full JSON grammar round-trip.
  Round-trip tests use `Code.eval_string/1` wrapping generated code in a fresh module.

## [0.3.0] - 2026-03-23

### Added
- `error_definitions: []` field on `%TokenGrammar{}` struct for error-recovery patterns.
- Parsing support for `errors:` section in `.tokens` files (mirrors the `skip:` section — indented `NAME = /pattern/` lines are stored as error definitions).
- `validate_token_grammar/1` function — lint pass over a parsed `TokenGrammar` checking:
  duplicate names, empty patterns, invalid regexes, non-UPPER_CASE names/aliases,
  unknown mode (only `"indentation"` supported), unknown escape_mode (only `"none"` supported).
  Applies the same checks to `skip_definitions` and `error_definitions`.
- `validate_parser_grammar/2` function on `ParserGrammar` — lint pass checking:
  duplicate rule names, non-lowercase rule names, undefined rule references,
  undefined token references (when `token_names` MapSet provided), and unreachable rules
  (first rule is exempt as the start symbol). Synthetic tokens (`NEWLINE`, `INDENT`, `DEDENT`,
  `EOF`) are always valid.
- Updated `CrossValidator.validate/2` to use `TokenGrammar.token_names/1` helper for
  building the valid token set. Unused token detection now accounts for aliases (if the
  grammar references `STRING` and a definition has `alias: "STRING"`, that definition is
  considered used).
- `Mix.Tasks.GrammarTools.Validate` Mix task (`mix grammar_tools.validate`) with three
  subcommands: `validate`, `validate_tokens`, `validate_grammar`. Output format matches
  the Python CLI exactly (e.g., `OK (N tokens, M skip, K error)`).
- Trace mode (`trace: true` option) in `CodingAdventures.Parser.GrammarParser.parse/3` —
  emits `[TRACE] rule 'name' at token N (TYPE "value") → match|fail` lines to stderr
  for each rule attempt, aiding parse failure diagnosis. Does not affect parse results.
- Comprehensive test coverage for all new features (32 new tests across grammar_tools and
  parser packages).

## [0.2.0] - 2026-03-21

### Added
- Pattern group support: `group NAME:` sections in `.tokens` files for context-sensitive lexing.
- `groups` field on `TokenGrammar` struct — a map from group name to pattern group (with `name` and `definitions`).
- `effective_token_names/1` function — returns token names as the parser sees them (aliases replace original names).
- `token_names/1` now includes names from all pattern groups and handles aliases.
- Group name validation: must match `[a-z_][a-z0-9_]*`, rejects reserved names (`default`, `skip`, `keywords`, `reserved`, `errors`), rejects duplicates.
- Group definition parsing: same `NAME = /pattern/` or `NAME = "literal"` format as other sections, with `-> ALIAS` support.
- Comprehensive test coverage for pattern groups (parsing, aliases, error cases).

## [0.1.0] - 2026-03-20

### Added
- Initial release — port of the Python grammar-tools package to Elixir.
- `TokenGrammar` module: parses `.tokens` files into structured data.
- `ParserGrammar` module: parses `.grammar` files (EBNF notation).
- `CrossValidator` module: validates token/grammar cross-references.
- Full extended format support: skip, aliases, reserved, mode directives.
