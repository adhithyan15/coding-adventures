# Changelog

All notable changes to the Ruby build tool are documented in this file.

## [0.3.0] - 2026-03-29

### Added

- **TypeScript dependency resolution**: `parse_typescript_deps()` in `resolver.rb` parses `package.json` for `@coding-adventures/` scoped dependencies. `build_known_names()` maps TypeScript packages to npm scoped names.
- **Rust dependency resolution**: `parse_rust_deps()` parses `Cargo.toml` `[dependencies]` sections with `path =` references. `build_known_names()` maps Rust packages to their crate names.
- **Swift dependency resolution**: `parse_swift_deps()` parses `Package.swift` `.package(path: "../dep-name")` references. `build_known_names()` maps Swift packages to their directory names.
- **Library-over-program priority** in `build_known_names()`: library packages take priority over programs for the same ecosystem dependency name, preventing self-loop resolution.
- **Elixir enhancement**: `build_known_names()` now reads the actual `app:` atom from `mix.exs` for accurate resolution, in addition to the convention-based name.
- **Missing language extensions in `hasher.rb`**: `SOURCE_EXTENSIONS` and `SPECIAL_FILENAMES` now cover TypeScript (`.ts`, `.tsx`, `.json`; `package.json`, `tsconfig.json`, `vitest.config.ts`), Rust (`.rs`, `.toml`; `Cargo.toml`, `Cargo.lock`), Elixir (`.ex`, `.exs`; `mix.exs`, `mix.lock`), and Starlark (`.star`).
- **`build_content`, `is_starlark`, `declared_srcs`, `declared_deps` fields** on `Package` struct in `discovery.rb` for Starlark BUILD file support.
- **`build_content` populated in `walk_dirs`**: raw BUILD file text is now read during discovery for Starlark detection.
- **`affected_set` parameter** on `Executor.execute_builds()`: packages not in the affected set are now skipped (status `"skipped"`) in git-diff mode.
- **Git diff step** in `build.rb` CLI: step 5 now runs `GitDiff.get_changed_files` and `map_files_to_packages`, computes transitive dependents, and expands prerequisites. Packages outside the affected set are efficiently skipped.
- **`--diff-base` flag**: specifies the git ref to diff against (default: `origin/main`).
- **`--detect-languages` flag**: outputs `needs_<lang>=true|false` for all languages to stdout and `$GITHUB_OUTPUT`. Go is always included.
- **Expanded `--language` choices**: now accepts `typescript`, `rust`, `elixir`, `lua`, `perl`, `swift` in addition to `python`, `ruby`, `go`.
- **`--emit-plan` implementation**: previously parsed but not implemented; now fully serializes the build plan (packages, edges, affected list, languages needed) to JSON and exits.
- **`--plan-file` implementation**: previously parsed but not implemented; now reads a pre-computed plan, reconstructs packages and graph, re-reads platform-specific BUILD files for non-Starlark packages, and runs the build.
- **Starlark evaluation step** in `build.rb`: after discovery, Starlark BUILD files are evaluated via `starlark_evaluator.rb` to extract declared targets, sources, and commands.
- **`ALL_LANGUAGES` constant**: canonical ordered list of all supported languages.
- **`SHARED_PREFIXES` constant**: narrows shared-file detection to `.github/workflows/ci.yml` only.
- **`expand_affected_set_with_prereqs()`**: ensures transitive prerequisites of affected packages are scheduled on fresh CI runners.
- **`emit_build_plan()`, `run_from_plan()`, `detect_needed_languages()`, `compute_languages_needed()`, `output_language_flags()`, `rebuild_argv()` helpers** extracted for clarity.

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
