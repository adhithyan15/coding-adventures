# Changelog — grammar-tools (Go program)

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o output.go]` — compiles a `.tokens` file to Go source code.
- `compile-grammar <file.grammar> [-o output.go]` — compiles a `.grammar` file to Go source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- 8 new CLI tests for compile commands.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `cmd/grammar-tools/main.go` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and argument parsing.
- Exit codes 0/1/2 identical to all other language implementations.
