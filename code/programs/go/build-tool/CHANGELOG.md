# Changelog

All notable changes to the Go build tool will be documented in this file.

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
