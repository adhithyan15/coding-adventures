# Changelog

All notable changes to this program will be documented in this file.

## [1.0.0] - 2026-03-25

### Added

- Initial implementation of `capability-cage-generator`
- Reads `required_capabilities.json` and emits `gen_capabilities.go` with capabilities compiled in as Go constants
- `--manifest=<path>` flag for processing a single manifest file
- `--all` flag for sweeping all `code/packages/go/*/required_capabilities.json` files in the repo
- `--dry-run` flag for printing generated output without writing files
- Correct Go constant names (`cage.CategoryFS`, `cage.ActionRead`, etc.) for all 8 categories and 14 actions
- Package name derivation: reads `package` declaration from existing `.go` files, falls back to stripping hyphens from the manifest's `package` field
- Non-Go packages (python/, ruby/, etc.) are silently skipped with a stderr message
- Comprehensive test suite covering all flags, error conditions, and generated output format
