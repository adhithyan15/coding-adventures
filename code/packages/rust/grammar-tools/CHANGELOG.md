# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - F11 token-adjacency predicates (LANG84 E6)

### Added
- `.grammar` predicates `@space_before`, `@no_space_before`,
  `@newline_before`, `@no_newline_before`, `@space_after_next` and
  `@no_space_after_next`: zero-width elements that test the gap in front of
  the next token (`code/specs/F11-grammar-token-adjacency-predicates.md`).
  LANG84 §E6's examples parse verbatim, including `!@space_after_next`.
- `GrammarElement::AdjacencyPredicate { predicate }` and the
  `AdjacencyPredicate` enum (`name`, `variant_name`, `from_name`,
  `reads_after_next`, `holds`, `Display` as `@name`).
- `compile_parser_grammar` emits predicates with the fully qualified path
  `grammar_tools::parser_grammar::AdjacencyPredicate::…`, so the generated
  `use` line is unchanged.
- Validation: a nullability analysis (`nullable_rules`, a least fixpoint over
  the rules) reports every repetition (`{ }`, `{ }+`, `{ x // sep }`) whose
  body can match without consuming a token. It is an error when the body
  involves an adjacency predicate (directly or through referenced rules) and
  a `Warning:` otherwise, because 12 existing `csharp*.grammar` files
  (`switch_block`) already have such a body. The message says what the
  parser does (stops after the first empty iteration). This replaces a first
  version that only rejected bodies where *every* path was zero-width and so
  passed `{ @space_before | NAME }` (security review).
- 17 tests: every predicate by name, the truth table, the LANG84 examples,
  predicates after tokens and under `&`/`!`/`[ ]`/`{ }+`, error messages, no
  reference side effects, validation both ways, and compiler emission.

### Changed
- `@` is now meaningful in rule bodies. Unknown names are an error that lists
  the valid ones; `@asi(...)` is rejected as "not implemented yet"; LANG84 E7
  binding forms such as `NAME@declare` are rejected as unknown predicates.
  The existing "unexpected character" test now uses `$`.

### Notes
- Grammars without predicates are byte-identical: regenerating all 94 Rust
  compiled grammars that `generate-rust-compiled-grammars` can build gives the
  same bytes before and after this change.
- The 14 non-Rust grammar-tools ports already reject `@` ("unexpected
  character"), which satisfies LANG84's must-reject rule; clearer messages and
  real ports are follow-ups recorded in F11.

## [Unreleased] - F10 declarative lexer mode transitions

### Added
- `ModeTransition` and `TransitionAction` types, plus `start_mode` and
  `transitions` fields on `TokenGrammar`, implementing the declarative
  lexer-mode transition table from `F10-declarative-lexer-modes.md`. A
  grammar can now drive context-sensitive lexing (regex-vs-division,
  template substitutions) from the `.tokens` file instead of a
  hand-written host-language callback.
- `.tokens` DSL: a `start_mode:` directive and a `transitions:` section
  whose lines read `on TOKENS [in MODE] -> ACTION [, ACTION ...]`
  (actions: `set-mode`/`push`/`pop`/`enable-skip`/`disable-skip`;
  `KEYWORD="value"` value-guard form supported).
- `parse_transition` parser, validation (undefined target/guard modes are
  rejected; `start_mode` must resolve; `MAX_TRANSITIONS` cap as a DoS
  guard), and `transitions_src`/`transition_src`/`action_src` codegen in
  `compiler.rs` (emits the table as pure data, mirroring `groups_src`).

### Changed
- All compiled `_grammar.rs` artifacts gain the two new fields as trivial
  defaults (`start_mode: None, transitions: vec![]`); behaviour is
  byte-identical to before for any grammar that declares no transitions
  (full F04 backward-compat).

### Notes
- Interpreting the table in the lexer is a separate change (F10 step 2,
  `grammar_lexer.rs`). This entry covers the grammar-tools core: types,
  parse, validation, and codegen.

## [0.6.0] - LS04 spec dump

### Added
- New `dump_spec` module exposing `dump_language_spec(token_grammar,
  parser_grammar, metadata)` — serialises a parsed grammar pair into
  the LanguageSpec v1 JSON document consumed by VS Code extension
  generation, treesitter wrappers, and other editor tooling
  (LS04).
- `infer_brackets()` helper — derives bracket pairs from conventional
  token names (`LPAREN`/`RPAREN`, `LBRACE`/`RBRACE`, etc.).
- `serde_json` dependency for the dump-spec JSON output (regular dep,
  no derive macros — keeps the dep tree small).

### Notes
- `<lang>-parser` crates can pair this with their build-time-compiled
  `ParserGrammar` and the lexer crate's `TokenGrammar` to expose a
  `<lang>-spec-dump` binary; see `code/packages/rust/twig-parser/bin/twig_spec_dump.rs`
  for the pattern.

## [0.5.0] - 2026-04-04

### Added
- `TokenGrammar.context_keywords: Vec<String>` field for context-sensitive
  keywords (words that are keywords in some syntactic positions but
  identifiers in others, like JavaScript's `async`, `await`, `get`, `set`).
- `context_keywords:` section parsing in `parse_token_grammar` — each
  indented line in the section is collected as a context keyword.
- Four new `GrammarElement` variants:
  - `PositiveLookahead { element }` — `&element` syntax; succeeds without
    consuming input if element matches.
  - `NegativeLookahead { element }` — `!element` syntax; succeeds without
    consuming input if element does NOT match.
  - `OneOrMore { element }` — `{ element }+` syntax; one-or-more repetition.
  - `SeparatedRepetition { element, separator, at_least_one }` —
    `{ element // separator }` syntax; separated repetition.
- Tokenizer: `Ampersand`, `Bang`, `Plus`, `DoubleSlash` token kinds for
  the new grammar syntax.
- Parser: `parse_element` handles `&`, `!` prefix operators, `+` suffix,
  and `//` separator inside braces.
- Compiler: `element_src` generates Rust code for all new element variants.
- `collect_token_refs` and `collect_rule_refs` updated for new variants.

## [0.4.0] - 2026-03-26

### Added
- `src/compiler.rs` — Grammar compiler module with:
  - `compile_token_grammar(grammar: &TokenGrammar, source_file: &str) → String` — generates
    Rust source with a `pub fn token_grammar() -> TokenGrammar` function.
  - `compile_parser_grammar(grammar: &ParserGrammar, source_file: &str) → String` — generates
    Rust source with a `pub fn parser_grammar() -> ParserGrammar` function.
  - All `GrammarElement` variants: `RuleReference`, `TokenReference`, `Literal`, `Sequence`,
    `Alternation`, `Repetition`, `Optional`, `Group`.
  - `rust_string_lit` uses raw strings (`r#"..."#`) to avoid backslash clutter in patterns;
    falls back to escaped strings if the value contains `"#`.
  - Groups rendered as inline `HashMap` construction block.
- `compiler` module exported from `lib.rs` as `pub mod compiler`.
- 25 inline tests in `src/compiler.rs` covering header, field content, and all element types.

## [0.3.0] - 2026-03-23

### Added

- `src/bin/grammar-tools.rs` — CLI binary exposing three subcommands:
  - `grammar-tools validate <file.tokens> <file.grammar>` — validate a pair of files
  - `grammar-tools validate-tokens <file.tokens>` — validate only the tokens file
  - `grammar-tools validate-grammar <file.grammar>` — validate only the grammar file
  - `grammar-tools --help` — print usage information
- Output format mirrors the Python `python -m grammar_tools` implementation:
  - `Validating <file> ... OK (N tokens, M skip, K error)` on success
  - `Validating <file> ... P error(s)` with indented details on failure
  - `All checks passed.` / `Found N error(s). Fix them and try again.` summary
- Exit codes: 0 = success, 1 = validation errors, 2 = usage errors
- `[[bin]]` entry added to `Cargo.toml` (`name = "grammar-tools"`)
- `tests/cli_tests.rs` — 14 integration tests invoking the binary via
  `std::process::Command`, covering all subcommands, missing files, wrong
  argument counts, unknown commands, and cross-validation warnings

## [0.2.0] - 2026-03-21

### Added

- `PatternGroup` struct for named sets of token definitions (context-sensitive lexing)
- `groups: HashMap<String, PatternGroup>` field on `TokenGrammar`
- `group NAME:` section parsing in `parse_token_grammar()` with validation:
  - Group name must match `[a-z_][a-z0-9_]*`
  - Reserved names (`default`, `skip`, `keywords`, `reserved`, `errors`) rejected
  - Duplicate group names rejected
  - Group definitions use same definition parser as other sections
- `effective_token_names()` function (returns alias names where present)
- `token_names()` now includes names from all pattern groups
- Group validation in `validate_token_grammar()`: bad regex, empty groups, naming conventions
- Comprehensive test suite for pattern groups (parsing, validation, error cases)

## [0.1.0] - 2026-03-19

### Added

- `token_grammar` module: parse `.tokens` files into `TokenGrammar` structs
  - `TokenDefinition` struct with name, pattern, is_regex, and line_number
  - `TokenGrammar` struct with definitions and keywords
  - `parse_token_grammar()` function with detailed error messages
  - `validate_token_grammar()` lint pass for duplicates, invalid regex, naming conventions
  - `token_names()` helper to extract defined token names
- `parser_grammar` module: parse `.grammar` files (EBNF) into `ParserGrammar` structs
  - `GrammarElement` enum with 8 variants: RuleReference, TokenReference, Literal, Sequence, Alternation, Repetition, Optional, Group
  - `GrammarRule` and `ParserGrammar` structs
  - Hand-written recursive descent parser (tokenizer + parser)
  - `validate_parser_grammar()` for undefined references, duplicates, unreachable rules
  - `rule_names()`, `grammar_token_references()`, `grammar_rule_references()` helpers
- `cross_validator` module: check consistency between `.tokens` and `.grammar` files
  - Reports missing token references as errors
  - Reports unused tokens as warnings
- Comprehensive test suite covering parsing, validation, error cases, and cross-validation
