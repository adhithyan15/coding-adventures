# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`BuildTool.GlobMatch`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns.
- **Strict input filtering in git diff**: `BuildTool.GitDiff.map_files_to_packages/4` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`BuildTool.Plan`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan/2` and `read_plan/1` with version checking.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.
- **`BUILD_windows`**: Windows-compatible BUILD file without shell redirects.

## [0.1.0] - 2026-03-21

### Added

- Initial release: full port of the Go build tool to Elixir.
- `BuildTool.Discovery` — recursive BUILD file discovery with skip list and platform-specific BUILD files.
- `BuildTool.DirectedGraph` — inline directed graph with Kahn's algorithm for topological levels and affected-node computation.
- `BuildTool.Resolver` — dependency resolution for Python, Ruby, Go, TypeScript, Rust, and Elixir packages.
- `BuildTool.GitDiff` — git-based change detection using three-dot diff with two-dot fallback.
- `BuildTool.Hasher` — deterministic SHA256 hashing of source files and transitive dependency hashes.
- `BuildTool.Cache` — Agent-based JSON build cache with atomic writes.
- `BuildTool.Executor` — parallel build execution by dependency level using `Task.async_stream`.
- `BuildTool.Reporter` — fixed-width terminal report table.
- `BuildTool.CLI` — escript entry point with the same flags as the Go build tool.
- Progress bar integration via the `CodingAdventures.ProgressBar` package.
- Comprehensive test suite covering all modules.
