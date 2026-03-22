# Changelog

All notable changes to the TypeScript build tool will be documented in this file.

## [1.0.0] - 2026-03-21

### Added

- Initial release: complete port of the Python build tool to TypeScript.
- Package discovery via recursive BUILD file walk (`discovery.ts`).
- Platform-specific BUILD file support: `BUILD_mac`, `BUILD_linux`, `BUILD_windows`, `BUILD_mac_and_linux`.
- Dependency resolution for all 6 languages: Python, Ruby, Go, TypeScript, Rust, Elixir (`resolver.ts`).
- Inline DirectedGraph implementation with Kahn's algorithm for topological sorting.
- Git-based change detection using `git diff --name-only` (`gitdiff.ts`).
- SHA256 file hashing for cache-based change detection (`hasher.ts`).
- JSON build cache with atomic writes (`cache.ts`).
- Parallel build execution respecting dependency order (`executor.ts`).
- Human-readable build report formatting (`reporter.ts`).
- CLI entry point with `--root`, `--force`, `--dry-run`, `--jobs`, `--language`, `--diff-base`, and `--cache-file` options (`index.ts`).
- Comprehensive test suite with >80% coverage.
- Zero runtime dependencies -- only Node.js built-in modules.
- Knuth-style literate programming throughout all source files.
