# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-03-22

### Added

- **Glob matching module** (`glob_match.py`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns. Used for strict input filtering and source file resolution.
- **Strict input filtering in git diff**: `map_files_to_packages()` now accepts optional `packages` parameter. For Starlark packages with `declared_srcs`, only files matching declared source patterns (or BUILD files) trigger rebuilds. Non-source file changes (README, CHANGELOG) no longer cause spurious rebuilds.
- **Build plan module** (`plan.py`): Serializes/deserializes build plan as versioned JSON (`schema_version: 1`). Supports `write_plan()` and `read_plan()` with version checking and forward compatibility.
- **`--emit-plan` CLI flag**: Writes build plan JSON to a file and exits.
- **`--plan-file` CLI flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff.
- **`is_starlark`, `declared_srcs`, `declared_deps` fields** on `Package` dataclass for Starlark BUILD file support.

### Fixed

- **Windows path normalization**: `map_files_to_packages()` normalizes backslash paths to forward slashes for consistent prefix matching against git diff output.

## [0.1.0] - 2026-03-18

### Added
- Initial implementation of the monorepo build tool
- Package discovery via recursive DIRS/BUILD file walking
- Platform-specific BUILD file support (BUILD_mac, BUILD_linux)
- Dependency resolution from pyproject.toml (Python), .gemspec (Ruby), go.mod (Go)
- SHA256-based file hashing for change detection
- JSON-based build cache with atomic writes
- Parallel execution via ThreadPoolExecutor, respecting dependency order
- Dependency-skip propagation: if a package fails, dependents are skipped
- Build report with status summary table
- CLI with --root, --force, --dry-run, --jobs, --language, --cache-file options
- Auto-detection of repository root via .git directory
- Test fixtures for simple (single package) and diamond (A->B->D, A->C->D) topologies
