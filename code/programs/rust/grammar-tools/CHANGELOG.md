# Changelog — grammar-tools (Rust program)

## [2.1.0] - 2026-03-26

### Added
- `--force` / `-f` flag — skip grammar validation and compile anyway. Used when patterns
  (e.g. lookahead `(?!...)` in `excel.tokens`, `xml.tokens`) are known to be handled at
  runtime by the consuming package. Without `--force`, any unsupported regex pattern causes
  the compile command to exit 1.

## [2.0.0] - 2026-03-26

### Added
- `compile-tokens <file.tokens> [-o out.rs]` — compiles a `.tokens` file to Rust source code.
- `compile-grammar <file.grammar> [-o out.rs]` — compiles a `.grammar` file to Rust source code.
- `--output` / `-o` flag — write generated code to a file instead of stdout.
- Status messages go to stderr; generated code goes to stdout.
- 8 new tests for compile commands and dispatch routing.

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `src/bin/grammar-tools.rs` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and parsing.
- Exit codes 0/1/2 identical to all other language implementations.
