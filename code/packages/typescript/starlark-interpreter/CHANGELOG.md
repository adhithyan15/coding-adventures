# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- `StarlarkInterpreter` class with `interpret()`, `interpretBytecode()`, and `interpretFile()` methods.
- `load()` support via LOAD_MODULE opcode override with file resolver pattern.
- Load caching: each file evaluated at most once, matching Bazel semantics.
- `FileResolver` type: function resolver, dict resolver, or null.
- `dictResolver()` convenience function for testing.
- `resolveFile()` internal helper for normalizing resolver types.
- `FileNotFoundError` for load resolution failures.
- Module-level convenience functions: `interpret()`, `interpretBytecode()`, `interpretFile()`.
- `Op` constant object with all Starlark opcodes.
- Mini Starlark VM (`createMiniStarlarkVM`, `registerMiniStarlarkHandlers`) for testing without the full starlark-vm package.
- Pluggable `compileFn` and `createVMFn` for dependency injection.
- Comprehensive test suite with 30+ tests covering bytecode execution, file resolvers, load caching, error handling, and all mini VM handlers.
