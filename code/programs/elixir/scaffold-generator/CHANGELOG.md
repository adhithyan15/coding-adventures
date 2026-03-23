# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial Elixir implementation of scaffold-generator
- Name normalization: to_snake_case, to_camel_case, to_joined_lower, dir_name
- Dependency reading for all 6 languages (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Transitive closure via BFS
- Topological sort via Kahn's algorithm
- File generation for all 6 languages
- CLI argument parsing using OptionParser
- Comprehensive ExUnit test suite
