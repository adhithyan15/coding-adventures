# Changelog

All notable changes to the Go build tool will be documented in this file.

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
