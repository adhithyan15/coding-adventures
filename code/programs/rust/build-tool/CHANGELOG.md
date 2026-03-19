# Changelog

All notable changes to the Rust build tool will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
