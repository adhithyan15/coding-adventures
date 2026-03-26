# Changelog — grammar-tools (Lua program)

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.lua]` — compiles a `.tokens` file to
  Lua source code that embeds the grammar as native Lua data structures.
- `compile-grammar <file.grammar> [-o out.lua]` — compiles a `.grammar` file
  to Lua source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages go to stderr; generated code goes to stdout.
- 8 new tests for compile commands and dispatch routing.

## [1.0.0] - 2026-03-26

### Added
- Initial release.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Plain `arg[n]` argument parsing (no cli-builder package available for Lua).
- Exit codes 0/1/2 identical to all other language implementations.
