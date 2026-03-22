# Changelog

All notable changes to this project will be documented in this file.

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
