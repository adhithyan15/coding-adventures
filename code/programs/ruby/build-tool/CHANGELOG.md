# Changelog

All notable changes to the Ruby build tool are documented in this file.

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
