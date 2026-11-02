# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- Haskell scaffolds now emit a `BUILD` that selects the Command Line Tools
  linker on macOS: the two-line explanation, then
  `if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal test all`.
  GHC 9.4.8's macOS bindist hardcodes a CLT SDK sysroot that an older default
  Xcode linker cannot read (`arm64e.x1`), and `build-tool --validate-build-files`
  rejects a Haskell BUILD without the selection.
- Haskell scaffolds now also emit a `BUILD_windows` holding the plain
  `cabal test all`, because `cmd /C` cannot parse the POSIX prefix. Windows
  previously ran the plain `BUILD`, so its behaviour is unchanged. Tests pin
  both files exactly (spec section 5.8).

## [1.0.0] - 2026-03-21

### Added

- Initial Rust implementation of scaffold-generator
- Manual argument parsing (no external dependencies, no cli-builder)
- Name normalization: to_snake_case, to_camel_case, to_joined_lower
- Dependency reading for all 6 languages (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Transitive closure via BFS
- Topological sort via Kahn's algorithm for correct BUILD install ordering
- File generation for all 6 languages with templates matching Go implementation
- Rust workspace Cargo.toml auto-update
- Dry-run mode
- Integration tests for name normalization, argument parsing, dependency resolution, and file generation
