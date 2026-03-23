# Changelog

## 0.1.0 — 2026-03-23

### Added
- Initial Lua scaffold generator implementation.
- Generates rockspec, BUILD, BUILD_windows, init.lua, test stub, README, CHANGELOG.
- Dependency resolution: transitive closure via BFS, topological sort via Kahn's.
- CLI with --depends-on, --layer, --description, --type, --dry-run options.
- Name normalization: kebab-case input → snake_case dirs, coding-adventures-* rockspecs.
