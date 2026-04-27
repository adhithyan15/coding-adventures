# Changelog

All notable changes to the Rust build tool will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- **`_ctx` build context injection** (`starlark.rs`): The Starlark evaluator now injects a `_ctx` dict into every BUILD file evaluation. The dict carries `os` (e.g. `"macos"`, `"linux"`, `"windows"`) and `arch` (e.g. `"x86_64"`, `"arm64"`) keys, enabling OS-aware rule logic in BUILD files (Phase 8: OS-Aware Starlark BUILD Rules).
- **`commands` field on `Target` struct**: Every resolved build target now carries an optional `Vec<String>` of rendered shell command strings.
- **`render_command(cmd)`**: Converts a single Starlark command dict (`executable` + `args` fields) into a quoted shell string.
- **`render_commands(cmds)`**: Maps a list of command dicts through `render_command()` and returns a `Vec<String>` of shell strings.
- **`quote_arg(arg)`**: Shell-safe quoting helper — wraps arguments containing spaces or special characters in double quotes.
- **`starlark-interpreter` and `virtual-machine` crate dependencies**: The build tool now links against the workspace `starlark-interpreter` and `virtual-machine` crates for Starlark evaluation and VM-based value handling.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`glob_match.rs`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns.
- **Strict input filtering in git diff**: `map_files_to_packages()` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds.
- **Build plan module** (`plan.rs`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan()` and `read_plan()` with version checking.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.

## [0.1.0] - 2026-03-19

### Added

- **Complete Rust port** of the Go build tool with identical behavior and algorithms.
- **Package discovery** via recursive DIRS/BUILD file walking. Supports platform-specific BUILD files (BUILD_mac, BUILD_linux) with automatic fallback to generic BUILD.
- **Dependency resolution** for Python (pyproject.toml), Ruby (.gemspec), Go (go.mod), and Rust (Cargo.toml). Internal dependencies are mapped using ecosystem-specific naming conventions.
- **SHA256 content hashing** for incremental builds. Two-level hashing: individual files are hashed, then all hashes are concatenated and hashed again. Language-aware file filtering.
- **Dependency hashing** to propagate changes through the dependency tree. If a transitive dependency changes, all dependents are rebuilt.
- **JSON-based build cache** (.build-cache.json) with atomic writes via temporary file + rename.
- **Git-based change detection** as the default mode. Uses three-dot diff (`base...HEAD`) with fallback to two-dot diff.
- **Parallel execution** using Rayon's work-stealing thread pool. Packages are built in topological levels -- packages in the same level run in parallel.
- **Failure propagation** -- if a package fails, all transitive dependents are marked "dep-skipped".
- **Build report** with aligned columns showing package name, status, and duration. Summary line shows counts by status category.
- **CLI flags**: --root, --diff-base, --force, --dry-run, --jobs, --language, --cache-file (using clap with derive feature).
- **Language filtering** to build only Python, Ruby, Go, Rust, or all packages.
- **Embedded directed graph** implementation with topological sort, independent groups, and affected-node queries.
- Comprehensive unit tests across all eight modules.
- Knuth-style literate comments throughout the codebase explaining design decisions.
- Cross-platform BUILD command execution (sh -c on Unix, cmd /C on Windows).

### Dependencies

- clap 4 (CLI parsing with derive macros)
- rayon 1.10 (parallel execution)
- serde + serde_json 1 (JSON cache serialization)
- sha2 0.10 (SHA256 hashing)
- toml 0.8 (Cargo.toml and pyproject.toml parsing)
- num_cpus 1.16 (CPU count detection)
