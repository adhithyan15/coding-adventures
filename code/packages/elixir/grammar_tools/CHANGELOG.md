# Changelog

## [0.6.1] - 2026-08-17

### Fixed
- `Compiler.element_src/2` had no clauses for `:positive_lookahead`,
  `:negative_lookahead`, `:one_or_more`, or `:separated_repetition` — legal
  `ParserGrammar` element types already handled by the runtime
  `GrammarParser` and by the other language ports' compilers, but
  `compile-grammar` crashed with `FunctionClauseError` on any grammar using
  them (e.g. `ruby.grammar`'s `!"rescue"`-style negative lookahead,
  `sql.grammar`'s `!("FOREIGN" "KEY")`, `algol60.grammar`'s
  `&(SEMICOLON)`, and every `fsharp<version>.grammar`'s
  `list_expression` rule). Added the four missing clauses, mirroring the
  TypeScript compiler and the runtime parser's tagged-tuple shapes.
- `Compiler.compile_token_grammar/2` rendered `keywords`, `reserved_keywords`,
  and `layout_keywords` (and `mode_transition_src/2`'s `on_tokens`) with
  plain `inspect/1`, which silently truncates lists past Elixir's default
  50-item limit (e.g. `sql.tokens`'s 50+ keyword list came out as
  `[..., ...]`, an invalid/wrong literal). Added `limit: :infinity` to
  every list-valued `inspect/1` call in the compiler.
- Corrected `mix.exs`'s `version` field, which had drifted to `0.1.0` while
  this CHANGELOG had already reached `0.5.0`/`0.6.0` — every prior release
  from 0.2.0 onward was never actually reflected in the published version.

## [0.6.0] - 2026-06-14

### Added (F10 — declarative lexer mode transitions)
- **`start_mode` field** on `%TokenGrammar{}` — the lexer mode (active group)
  the tokenizer starts in. `nil` means `"default"`. Set by the new
  `start_mode: NAME` directive in `.tokens` files.
- **`transitions` field** on `%TokenGrammar{}` — a list of `mode_transition`
  maps representing declarative mode transition rules. Empty means no
  transitions (pre-F10 behaviour). Set by the new `transitions:` section.
- **`transition_action` type** — tagged tuples `{:set_mode, name}`,
  `{:push, name}`, `:pop`, `:enable_skip`, `:disable_skip`.
- **`mode_transition` type** — map with `on_tokens`, `on_value`, `in_mode`,
  `actions`, and `line_number`.
- **`@max_transitions 4096`** — safety cap on transition rule count; enforced
  in `validate_token_grammar/1`.
- **Parsing** for `start_mode:` directive and `transitions:` section with
  indented `on TOKENS [in MODE] -> ACTION [, ACTION ...]` entries. Supports:
  - Single token: `on SLASH -> set-mode div`
  - Alternation: `on (SLASH | STAR) -> set-mode default`
  - Keyword-value guard: `on KEYWORD="return" -> set-mode default`
  - In-guard: `on SLASH in default -> set-mode div`
  - Multiple actions (comma-separated): `on TOK -> set-mode g, enable-skip`
- **Validation** in `validate_token_grammar/1`:
  - `start_mode` must be `"default"` or a declared group.
  - `in MODE` guards must name a declared group (or `"default"`).
  - `set-mode M` / `push G` targets must name a declared group (or `"default"`).
  - Enforces `MAX_TRANSITIONS` cap.
- **Compiler** (`compile_token_grammar/2`) now emits `start_mode:` and
  `transitions:` fields in the generated Elixir code, including all
  `transition_action` tagged tuples.
- **17 new tests** in `token_grammar_test.exs` covering backward compatibility,
  directive and section parsing, all 5 action kinds, error cases, and all
  F10 validation rules.

## [0.5.0] - 2026-04-04

### Added
- `TokenGrammar.context_keywords` field — list of context-sensitive keywords
- `context_keywords:` section parsing in `.tokens` files — words listed are
  emitted as NAME tokens with the `TOKEN_CONTEXT_KEYWORD` flag by the lexer
- `ParserGrammar` new grammar element types:
  - `{:positive_lookahead, element}` — `&element` syntax, matches without consuming
  - `{:negative_lookahead, element}` — `!element` syntax, succeeds if element fails
  - `{:one_or_more, element}` — `{ element }+` syntax, requires at least one match
  - `{:separated_repetition, element, separator, at_least_one}` — `{ element // separator }`
    syntax for comma-separated lists and similar patterns
- Tokenizer support for `&`, `!`, `+`, and `//` operators in `.grammar` files
- `collect_refs` handles all new element types for reference collection

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
