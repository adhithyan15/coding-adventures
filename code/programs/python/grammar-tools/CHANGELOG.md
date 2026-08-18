# Changelog — grammar-tools (Python program)

## [2.0.1] - 2026-08-17

### Added
- `--force` / `-f` flag on `compile-tokens`/`compile-grammar` — skip
  grammar validation and compile anyway (mirrors the flag the
  Ruby/Go/Rust/TypeScript/Elixir ports already had). Needed because
  several `.grammar` files in `code/grammars/` (csharp, java, ecmascript
  es2018+, typescript ts3.0+) have pre-existing "unreachable rule" or
  "undefined rule reference" validation warnings that otherwise block
  compilation of their downstream generated Python files.

### Fixed
- Corrected `pyproject.toml`'s `version` field, which had drifted to
  `1.0.0` while this CHANGELOG had already reached `2.0.0`.

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
