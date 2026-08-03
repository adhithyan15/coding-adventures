# Changelog

All notable changes to `coding_adventures_starlark_interpreter` will be documented in this file.

## [Unreleased]

### Fixed
- Imported functions now retain the globals of the module that defined them,
  including helpers and injected rule functions used by nested Starlark loads.
- Loaded symbols whose names begin with `r` or `b`, such as `rust_library`,
  preserve their full identifiers through compilation and execution.

## [0.1.0] - 2026-03-21

### Added
- Initial release
- `Interpreter` class with full Starlark pipeline (lex, parse, compile, execute)
- `load()` statement support via configurable `file_resolver`
- Module caching -- loaded files are compiled and executed only once
- `interpret(source)` method for interpreting source strings
- `interpret_file(path)` method for interpreting files from disk
- Module-level convenience methods: `StarlarkInterpreter.interpret()` and `.interpret_file()`
- Comprehensive test suite with 30+ tests covering:
  - Basic interpretation
  - Functions and control flow
  - Collections
  - Print capture
  - Load with dictionary resolver
  - Load caching
  - Load with imported functions
  - Load error handling
  - File interpretation with temporary files
  - BUILD file simulation
