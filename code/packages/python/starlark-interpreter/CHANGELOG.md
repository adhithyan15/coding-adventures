# Changelog

## 0.1.0 — 2026-03-20

### Added
- Full Starlark interpreter pipeline: source → lexer → parser → compiler → VM → result.
- `interpret(source, file_resolver=None)` — one-call API for executing Starlark source.
- `interpret_file(path, file_resolver=None)` — execute a Starlark file from disk.
- `StarlarkInterpreter` class with shared load cache across multiple interpret calls.
- `load()` built-in function for importing symbols from other Starlark files:
  - `load("//path/to/file.star", "symbol1", "symbol2")` — Bazel-style load.
  - Supports dict-based file resolvers (for testing) and callable resolvers (for production).
  - Loaded files are cached — each file is evaluated at most once.
  - Recursive loading supported (loaded files can themselves use `load()`).
- Keyword argument support via CALL_FUNCTION_KW (enables BUILD-file-style calls like
  `py_library(name="foo", deps=["bar"])`).
