# Changelog

All notable changes to this project will be documented in this file.

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
