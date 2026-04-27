# Changelog

All notable changes to the TypeScript build tool will be documented in this file.

## [Unreleased]

### Added
- **`_ctx` build context injection** (`starlark.ts`): The Starlark evaluator now injects a `_ctx` dict into every BUILD file evaluation. The dict carries `os` (e.g. `"macos"`, `"linux"`, `"windows"`) and `arch` (e.g. `"x86_64"`, `"arm64"`) keys, enabling OS-aware rule logic in BUILD files (Phase 8: OS-Aware Starlark BUILD Rules).
- **`commands` field on `Target` interface**: Every resolved build target now carries an optional list of rendered shell command strings.
- **`renderCommand(cmd)`**: Converts a single Starlark command object (`{executable, args}`) into a quoted shell string.
- **`renderCommands(cmds)`**: Maps a list of command objects through `renderCommand()` and returns a list of shell strings.
- **`quoteArg(arg)`**: Shell-safe quoting helper — wraps arguments containing spaces or special characters in double quotes.

## [1.1.0] - 2026-03-22

### Added

- **Glob matching module** (`glob-match.ts`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns.
- **Strict input filtering in git diff**: `mapFilesToPackages()` now respects Starlark `declaredSrcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`plan.ts`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `writePlan()` and `readPlan()` with version checking and forward compatibility.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.

### Fixed

- **Windows path splitting**: `inferLanguage()` now splits on both `/` and `\` for correct language detection on Windows where paths may use backslashes.

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
