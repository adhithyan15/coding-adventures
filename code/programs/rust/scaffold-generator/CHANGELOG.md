# Changelog

All notable changes to this package will be documented in this file.

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
