# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Fixed

- Haskell scaffolds now emit the complete shared schema-v1 capability manifest
  and keep their starter library and test source pure.
- Haskell generation rejects control characters before descriptions are
  interpolated into Cabal metadata, and the CLI rejects structural
  block-comment delimiters before any template is written.
- Refreshed the lockfile to use patched PostCSS and its aligned Nano ID
  resolution.

## [1.0.0] - 2026-03-21

### Added

- Initial TypeScript implementation of scaffold-generator
- Name normalization (toSnakeCase, toCamelCase, toJoinedLower)
- Dependency reading for all 6 languages (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Transitive closure computation via BFS
- Topological sort via Kahn's algorithm for correct install ordering
- File generation for all 6 languages matching Go/Python reference implementations
- CLI builder integration for argument parsing
- Comprehensive test suite covering all major features
