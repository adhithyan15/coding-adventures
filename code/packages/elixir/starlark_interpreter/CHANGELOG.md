# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-03-22

### Added
- `interpret/2` — Execute Starlark source code through the full pipeline
- `interpret_file/2` — Execute a Starlark file from disk
- `load()` support with file resolver and caching
- Map-based file resolvers for testing
- Function-based file resolvers for production
- Load caching — each file evaluated at most once (Bazel semantics)
- Configurable max recursion depth
- Pre-populated load cache support for sharing across interpret calls
- 60+ tests covering arithmetic, data types, control flow, functions,
  builtins, module loading, file execution, and complex programs
