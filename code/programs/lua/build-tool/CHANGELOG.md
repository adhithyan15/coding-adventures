# Changelog

## 0.1.0 — 2026-03-23

### Added
- Initial implementation of the Lua build tool.
- Package discovery via recursive BUILD file walk.
- Dependency resolution for all 7 supported languages:
  Python (pyproject.toml), Ruby (.gemspec), Go (go.mod),
  TypeScript (package.json), Rust (Cargo.toml), Elixir (mix.exs),
  Lua (.rockspec).
- Directed graph with Kahn's algorithm for topological sort.
- Sequential build execution with pass/fail tracking.
- Build report with summary statistics.
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux, BUILD_windows).
- CLI with --root, --dry-run, --language, --force options.
