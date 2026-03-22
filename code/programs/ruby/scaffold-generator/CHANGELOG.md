# Changelog

All notable changes to this package will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial Ruby implementation of the scaffold-generator program
- Name normalization: to_snake_case, to_camel_case, to_joined_lower
- Dependency reading for all 6 languages (Python, Go, Ruby, TypeScript, Rust, Elixir)
- Transitive closure computation via BFS
- Topological sort via Kahn's algorithm for correct install ordering
- File generation for all 6 languages with proper BUILD files
- Critical TypeScript safeguards: main points to src/index.ts, @vitest/coverage-v8 included
- Critical Ruby safeguard: dependencies required before own modules
- Rust workspace Cargo.toml auto-update
- Dry-run mode for previewing generated files
- Full CLI integration via cli-builder (scaffold-generator.json spec)
