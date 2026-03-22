# Changelog

All notable changes to this program will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial implementation of scaffold-generator in Go
- CLI parsing via cli-builder with scaffold-generator.json spec
- Name normalization (kebab-case to snake_case, CamelCase, joinedlower)
- Dependency resolution: transitive closure via BFS, topological sort via Kahn's algorithm
- File generation for all 6 languages: Python, Go, Ruby, TypeScript, Rust, Elixir
- Automatic Rust workspace Cargo.toml member list updates
- Dry-run mode for previewing generated files
- Input validation (kebab-case names, known languages, existing dependencies)
- Comprehensive test suite covering name normalization, dep resolution, file generation
