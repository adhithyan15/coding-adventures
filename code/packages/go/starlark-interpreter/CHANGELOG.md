# Changelog

All notable changes to the starlark-interpreter package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `StarlarkInterpreter` struct with full pipeline: source -> lexer -> parser -> compiler -> VM -> result
- `FileResolver` function type for resolving `load()` labels to source code
- `DictResolver` helper for creating resolvers from `map[string]string` (ideal for testing)
- `load()` support by overriding the VM's `LOAD_MODULE` opcode handler
- Module caching: loaded files are executed once and cached across calls
- `Interpret()` convenience function for one-shot execution
- `InterpretFile()` convenience function for executing `.star` files from disk
- `NewInterpreter()` constructor with functional options pattern
- `WithFileResolver()` option for configuring load() file resolution
- `WithMaxRecursionDepth()` option for configuring max call stack depth
- 55 tests covering basic interpretation, strings, functions, control flow, collections, builtins, print capture, load support, load caching, load with functions, load errors, InterpretFile, BUILD file simulation, error handling, and options
- Literate programming style with detailed documentation throughout
- BUILD file for CI integration
- README.md with usage examples
