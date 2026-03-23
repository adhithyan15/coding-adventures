# Changelog

## 0.1.0 — 2026-03-21

### Added
- `StarlarkValue` enum for runtime values (Int, Float, String, Bool, None, List, Dict, Tuple, Function, Iterator)
- `StarlarkBuiltins` with ~20 built-in functions (type, bool, int, float, str, len, list, dict, tuple, range, sorted, reversed, enumerate, zip, min, max, abs, all, any, repr, print)
- `StarlarkResult` for execution results (variables, output, traces)
- `execute_starlark()` convenience function for source-to-result in one call
- Comprehensive test suite for builtins and value types
