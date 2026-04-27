# Changelog

All notable changes to the Ruby build tool are documented in this file.

## [Unreleased]

### Added
- **`_ctx` build context injection** (`starlark.rb`): The Starlark evaluator now injects a `_ctx` dict into every BUILD file evaluation. The dict carries `os` (e.g. `"macos"`, `"linux"`, `"windows"`) and `arch` (e.g. `"x86_64"`, `"arm64"`) keys, enabling OS-aware rule logic in BUILD files (Phase 8: OS-Aware Starlark BUILD Rules).
- **`commands` field on `Target` struct**: Every resolved build target now carries an optional list of rendered shell command strings.
- **`render_command(cmd)`**: Converts a single Starlark command hash (`{executable:, args:}`) into a quoted shell string.
- **`render_commands(cmds)`**: Maps a list of command hashes through `render_command` and returns a list of shell strings.
- **`quote_arg(arg)`**: Shell-safe quoting helper — wraps arguments containing spaces or special characters in double quotes.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`glob_match.rb`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns. No filesystem access needed.
- **Strict input filtering in git diff**: `map_files_to_packages` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`plan.rb`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan` and `read_plan` with version checking.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.

## [0.1.0] - 2026-03-18

### Added

- Initial Ruby port of the Python build tool.
- `discovery.rb` -- Package discovery via DIRS/BUILD files with platform-specific BUILD file support (BUILD_mac, BUILD_linux).
- `resolver.rb` -- Dependency resolution from pyproject.toml, .gemspec, and go.mod with a self-contained DirectedGraph implementation (Kahn's topological sort).
- `hasher.rb` -- SHA256 file hashing for change detection with two-level hashing (per-file then combined) and dependency hash computation.
- `cache.rb` -- JSON-based build cache with atomic writes (write-to-tmp-then-rename) for safe incremental builds.
- `executor.rb` -- Parallel build execution via `Thread.new` + `Open3.capture3`, respecting topological build order with dep-skip propagation on failure.
- `reporter.rb` -- Human-readable build report table with status counts.
- `build.rb` -- CLI entry point with OptionParser flags: --root, --force, --dry-run, --jobs, --language, --cache-file.
- Minitest test suite with SimpleCov coverage for all 6 modules.
- Test fixtures: simple (single package) and diamond (4 packages with diamond dependency shape).
