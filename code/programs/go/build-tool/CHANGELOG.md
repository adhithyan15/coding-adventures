# Changelog

All notable changes to the Go build tool will be documented in this file.

## [0.4.0] - 2026-03-27

### Fixed

- **Starlark packages now use `BUILD_windows` overrides on Windows**: The plan
  loader previously skipped re-reading `BUILD_windows` for Starlark packages,
  causing those packages to use Starlark-generated commands even when a
  platform-specific shell override existed. For example, `elixir/arithmetic`
  has a Starlark `BUILD` (generates `mix test --cover`) and a shell
  `BUILD_windows` (uses `mix test` without `--cover`). On Windows, Erlang's
  code coverage module causes failures, so the `BUILD_windows` override is
  essential. The fix: if a platform-specific override (BUILD_windows, BUILD_mac,
  etc.) exists and differs from the generic BUILD file, always use it regardless
  of `is_starlark`. Only skip re-reading when the platform resolves to the
  generic BUILD file itself (which may be Starlark).

### Changed

- **Windows executor switched from `cmd /C` to `pwsh -Command`**: The Windows
  shell runner now uses PowerShell 7 instead of `cmd.exe`. This eliminates the
  long-standing path-corruption bug where `cmd.exe` would strip outer
  double-quotes from arguments, causing `uv pip install -e "../../../packages/foo"`
  to fail because the trailing `"` was URL-encoded as `%22` by uv. PowerShell
  handles double-quoted strings correctly — quotes are preserved and passed
  through to the child process, not stripped. PowerShell also supports the `&&`
  operator for fail-fast command chaining (same idiom as bash), and
  forward-slash paths work without modification. PowerShell 7 (`pwsh`) is
  pre-installed on all GitHub Actions Windows runners.

## [0.3.0] - 2026-03-22

### Added

- **Glob matching library** (`internal/globmatch/`): Pure string-matching glob utility supporting `**` (zero or more directory segments), `*`, `?`, and literal patterns. No filesystem access needed — matches patterns against path strings directly.
- **Strict input filtering in git diff**: `MapFilesToPackages()` now respects Starlark `declared_srcs` patterns. For Starlark packages, only files matching declared source patterns (or BUILD files) trigger rebuilds. Editing `README.md` in a Starlark package no longer causes a spurious rebuild.
- **Build plan artifact** (`internal/plan/`): Serializes discovery, resolution, and change detection results as a versioned JSON manifest (`schema_version: 1`). Enables CI detect job to compute the build plan once, upload as artifact, and have build jobs on all 3 platforms skip redundant computation.
- **`--emit-plan` flag**: Writes the build plan JSON to a file and exits. Used by CI detect job.
- **`--plan-file` flag**: Reads a previously emitted build plan, skipping discovery/resolution/diff. Used by CI build jobs.
- **Cross-platform plan loading**: When loading a plan on a different OS than the detect job, re-reads platform-specific BUILD files to get correct commands (e.g., Windows gets `BUILD_windows` commands instead of Linux shell syntax).

### Fixed

- **`**` glob patterns in hasher**: `resolveDeclaredSrcs()` now uses `filepath.WalkDir` + `globmatch.MatchPath` instead of `filepath.Glob`, which silently failed on `**` patterns.

### Changed

- CI workflow now uploads/downloads build plan artifact between detect and build jobs, eliminating duplicate discovery/resolution computation on each platform.

## [0.2.0] - 2026-03-22

### Added

- **`--detect-languages` flag**: Outputs which language toolchains CI needs based on git diff. Enables conditional toolchain installation in CI — only install Python if Python packages changed, etc. Go is always needed (build tool dependency).
- **Starlark BUILD file evaluation**: BUILD files can now be written in Starlark instead of shell. The build tool detects Starlark BUILD files (via `load()` or rule calls) and evaluates them through the Go starlark-interpreter.
- **Starlark evaluator** (`internal/starlark/evaluator.go`): Evaluates Starlark BUILD files, extracts targets with declared srcs/deps, generates shell commands from rule types.
- **Strict input hashing**: When a package has declared srcs (from Starlark BUILD), only those files are hashed for change detection. Falls back to extension-based collection for shell BUILD files.
- **12 rule types supported**: py_library, py_binary, go_library, go_binary, ruby_library, ruby_binary, ts_library, ts_binary, rust_library, rust_binary, elixir_library, elixir_binary.
- **"starlark" language support**: Discovery and hasher recognize "starlark" as a first-class language alongside python/go/ruby/typescript/rust/elixir.
- **TypeScript, Rust, Elixir extension mappings** in hasher (previously only python/ruby/go were mapped).

### Dependencies

- Added starlark-interpreter and its 10 transitive Go package dependencies via replace directives.

## [0.1.0] - 2026-03-18

### Added

- **Package discovery** via DIRS/BUILD file walking. Supports platform-specific BUILD files (BUILD_mac, BUILD_linux) with automatic fallback to generic BUILD.
- **Dependency resolution** for Python (pyproject.toml), Ruby (.gemspec), and Go (go.mod). Internal dependencies are mapped using ecosystem-specific naming conventions (coding-adventures-* for Python, coding_adventures_* for Ruby, module paths for Go).
- **SHA256 content hashing** for incremental builds. Two-level hashing: individual files are hashed, then all hashes are concatenated and hashed again. Language-aware file filtering (only relevant source extensions are included).
- **Dependency hashing** to propagate changes through the dependency tree. If a transitive dependency changes, all dependents are rebuilt.
- **JSON-based build cache** (.build-cache.json) with atomic writes via temporary file + rename. Cache records package hash, dependency hash, timestamp, and build status.
- **Parallel execution** using goroutines with semaphore-based concurrency limiting. Packages are built in topological levels — packages in the same level run in parallel.
- **Failure propagation** — if a package fails, all transitive dependents are marked "dep-skipped".
- **Build report** with aligned columns showing package name, status, and duration. Summary line shows counts by status category.
- **CLI flags**: -root, -force, -dry-run, -jobs, -language, -cache-file.
- **Language filtering** to build only Python, Ruby, Go, or all packages.
- Comprehensive test suite covering all six internal packages.
- Knuth-style literate comments throughout the codebase explaining design decisions.

### Dependencies

- Uses the `directed-graph` package from `code/packages/go/directed-graph` via Go module replace directive.
