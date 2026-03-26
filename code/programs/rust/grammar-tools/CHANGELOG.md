# Changelog — grammar-tools (Rust program)

## [1.0.0] - 2026-03-26

### Added
- Initial release. Replaces `src/bin/grammar-tools.rs` in the library package.
- `validate`, `validate-tokens`, `validate-grammar` commands.
- Uses `cli-builder` for `--help`, `--version`, and parsing.
- Exit codes 0/1/2 identical to all other language implementations.
