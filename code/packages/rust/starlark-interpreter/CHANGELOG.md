# Changelog

All notable changes to the `starlark-interpreter` crate will be documented in this file.

## [Unreleased]

### Added
- `starlark_value_to_value()` — converts a `StarlarkValue` (Starlark runtime type) back into a `Value` (VM type), enabling round-trip conversion between the two type systems (Phase 8: OS-Aware Starlark BUILD Rules).
- `value_to_starlark_value()` updated to handle the new `Value::List` and `Value::Dict` variants, recursively converting inner elements.
- `value_is_truthy()` updated: empty `List` and empty `Dict` are falsy; non-empty are truthy, consistent with Python/Starlark semantics.
- `value_type_name()` updated to return `"list"` for `Value::List` and `"dict"` for `Value::Dict`.

## [0.1.0] - 2026-03-22

### Added

- `StarlarkInterpreter` struct with configurable file resolver and recursion limit
- `interpret_bytecode()` - execute pre-compiled CodeObject bytecode
- `interpret()` - execute Starlark source code (via stub compiler)
- `interpret_file()` - execute a Starlark file from disk
- `FileResolver` trait for pluggable file resolution in `load()` calls
- `DictResolver` - in-memory file resolver for testing
- `FsResolver` - filesystem-based resolver for production use
- `InterpreterResult` with convenience accessors (`get_int`, `get_string`, `get_bool`)
- `InterpreterError` enum covering all pipeline stages
- Load caching: each file evaluated at most once (Bazel semantics)
- `load_module()` for explicit module loading with cache
- Stub compiler handling: assignments, arithmetic, print, booleans, None, variables
- Full opcode handler registration for Starlark bytecode (stack ops, arithmetic, comparisons, control flow, collections, I/O)
- Value conversion utilities between `Value` (VM) and `StarlarkValue` (Starlark runtime)
- 62 unit tests covering bytecode execution, stub compilation, resolver behavior, caching, error handling, and edge cases
