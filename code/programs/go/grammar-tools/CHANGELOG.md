# Changelog — grammar-tools (Go program)

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `cmd/grammar-tools/main.go` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and argument parsing.
- Exit codes 0/1/2 identical to all other language implementations.
