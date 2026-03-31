# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-03-29

### Added

- **TypeScript dependency resolution**: `_parse_typescript_deps()` parses `package.json` for `@coding-adventures/` scoped dependencies. `_build_known_names()` now maps TypeScript packages to their npm scoped names.
- **Rust dependency resolution**: `_parse_rust_deps()` parses `Cargo.toml` `[dependencies]` sections with `path =` references. `_build_known_names()` maps Rust packages to their crate names.
- **Swift dependency resolution**: `_parse_swift_deps()` parses `Package.swift` `.package(path: "../dep-name")` references. `_build_known_names()` maps Swift packages to their directory names.
- **Library-over-program priority** in `_build_known_names()`: when a library package and a program share the same ecosystem dependency name, the library entry takes priority. Prevents self-loop dep resolution for programs that depend on their own library.
- **Elixir enhancement**: `_build_known_names()` now reads the actual `app:` atom from `mix.exs` in addition to the convention-based name, ensuring accurate cross-package resolution.
- **`build_content` field** on `Package` dataclass: raw BUILD file text, populated during discovery, for Starlark detection in CLI.
- **Starlark evaluation step** in CLI (`cli.py`): after discovery, Starlark BUILD files are evaluated via `starlark_evaluator.py` to extract declared targets, sources, and build commands.
- **Expanded `--language` choices**: now includes `typescript`, `rust`, `elixir`, `lua`, `perl`, `swift` in addition to `python`, `ruby`, `go`.
- **`--detect-languages` standalone mode**: outputs `needs_<lang>=true|false` for all languages when used without `--emit-plan`. Writes to both stdout and `$GITHUB_OUTPUT`.
- **`ALL_LANGUAGES` constant**: canonical ordered list of all supported languages.
- **`SHARED_PREFIXES` constant**: narrows shared-file detection from any `.github/` path to only `.github/workflows/ci.yml`, avoiding full rebuilds for deployment-only workflow changes.
- **`_expand_affected_set_with_prereqs()`**: ensures transitive prerequisites of affected packages are also scheduled. Prevents failures on fresh CI runners where prerequisite BUILD steps materialize local dependency state.
- **`DirectedGraph.affected_nodes()`**: returns changed packages plus all their transitive dependents.
- **`DirectedGraph.edges()`**: returns all directed edges as (from, to) tuples for plan serialization.

### Fixed

- **Language detection output**: `_output_language_flags()` now uses `needs_<lang>` prefix (matching Go build tool) instead of `need_<lang>`. Also writes to `$GITHUB_OUTPUT` for GitHub Actions integration.
- **Shared-file detection**: narrowed from `startswith(".github/")` to exact match against `SHARED_PREFIXES` to avoid spurious full rebuilds on deployment workflow changes.

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
