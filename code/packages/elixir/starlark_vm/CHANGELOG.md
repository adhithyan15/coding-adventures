# Changelog

## 0.1.0 — 2026-03-22

### Added
- 59 opcode handlers covering all Starlark bytecode instructions
- 23 built-in functions (type, bool, int, float, str, len, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, hasattr, getattr, print)
- `StarlarkFunction` struct for compiled function objects
- `StarlarkIterator` struct for for-loop iteration
- `StarlarkResult` struct for execution results
- `create_starlark_vm/0` factory function
- `execute_starlark/1` convenience function for source-to-result
- String, list, and dict method support via LOAD_ATTR
- Truthiness implementation matching Starlark/Python semantics
- 80+ unit tests covering handlers, builtins, and end-to-end execution
- BUILD file, README.md, CHANGELOG.md
