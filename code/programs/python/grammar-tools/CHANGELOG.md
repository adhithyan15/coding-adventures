# Changelog — grammar-tools (Python program)

## [2.0.0] - 2026-03-26

### Added
- **`compile-tokens` command**: parses a `.tokens` file and generates Python source code
  embedding the `TokenGrammar` as native data structures. Output goes to stdout or to
  a file with `-o <path>`.
- **`compile-grammar` command**: same as above but for `.grammar` files, generating
  `ParserGrammar` source code.
- **`--output` / `-o` flag**: optional output file path for compile commands.
- 8 new CLI tests covering dispatch, output file writing, and exec-ability of generated code.

## [1.0.0] - 2026-03-26

### Added
- Initial release of the `grammar-tools` Python program.
- `validate` command: cross-validates a `.tokens`/`.grammar` pair.
- `validate-tokens` command: validates a `.tokens` file in isolation.
- `validate-grammar` command: validates a `.grammar` file in isolation.
- Built on `cli_builder` for `--help`, `--version`, and argument parsing.
- Wraps the `grammar_tools` library (unchanged).
- Exit codes: 0 (success), 1 (validation errors), 2 (usage errors).
- Output format identical to Elixir, Ruby, Go, Rust, TypeScript, Lua counterparts.

### Changed
- Replaces the `python -m grammar_tools` CLI (`__main__.py`) that previously
  lived in `code/packages/python/grammar-tools/`. The standalone program is now
  the canonical way to run grammar-tools from the command line.
