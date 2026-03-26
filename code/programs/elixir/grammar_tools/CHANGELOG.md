# Changelog — grammar-tools (Elixir program)

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
