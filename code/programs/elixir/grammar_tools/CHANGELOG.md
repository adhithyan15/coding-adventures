# Changelog — grammar-tools (Elixir program)

## [2.0.1] - 2026-08-17

### Added
- `--force` / `-f` flag on `compile-tokens`/`compile-grammar` — skip
  grammar validation and compile anyway (mirrors the flag the
  Ruby/Go/Rust/TypeScript ports already had). Needed because several
  `.grammar` files in `code/grammars/` (java1.0.grammar through
  java21.grammar — all 10 versions trip a pre-existing false-positive
  "unreachable rule" warning on their `compilation_unit` start rule) block
  compilation of their downstream `_grammar.ex` files without it.

### Fixed
- Corrected `mix.exs`'s `version` field, which had drifted to `1.0.0`
  while this CHANGELOG had already reached `2.0.0`.

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.ex]` — compiles a `.tokens` file to Elixir source code.
- `compile-grammar <file.grammar> [-o out.ex]` — compiles a `.grammar` file to Elixir source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages go to stderr; generated code goes to stdout.
- 8 new tests for compile commands and dispatch routing.

## [1.0.0] - 2026-03-26

### Added
- Initial release of the `grammar-tools` escript program.
- `validate <tokens> <grammar>` command — cross-validates a `.tokens`/`.grammar`
  pair in three steps: validate tokens, validate grammar, cross-validate both.
- `validate-tokens <tokens>` command — validates a `.tokens` file in isolation.
- `validate-grammar <grammar>` command — validates a `.grammar` file in isolation.
- Built on top of `CodingAdventures.CliBuilder` for argument parsing, `--help`,
  and `--version` support.
- Wraps `CodingAdventures.GrammarTools` library (unchanged) so all the actual
  validation logic is shared with the library package.
- Exit codes: 0 (success), 1 (validation errors), 2 (usage errors).
- Output format is identical to the Python, Ruby, Go, Rust, TypeScript, and Lua
  counterparts so CI scripts can use any implementation interchangeably.

### Changed
- Replaces the `Mix.Tasks.GrammarTools.Validate` Mix task that previously lived
  in `code/packages/elixir/grammar_tools/`. The escript is a standalone binary
  that works without Mix, making it consistent with all other language programs.
