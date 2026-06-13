# Changelog

All notable changes to `coding-adventures-sir-runtime-core` are documented here.

## [0.1.0] - 2026-06-11

### Added

Initial release — the core runtime imported by Semantic-IR-emitted Python.
Provides the SIR semantics that have no faithful native Python equivalent:

- **SIR truthiness** (`truthy`) — only `False` and `nil` are falsy (`0`, `""`,
  `[]`, `{}` are truthy), the Lisp/Ruby convention.
- **Symbols** (`Symbol`, `intern`) — interned identity objects.
- **Pairs** (`Pair`, `cons`, `car`, `cdr`, `is_pair`) — cons cells with Lisp
  list display.
- **Equality / predicates** (`eq`, `is_null`, `is_number`, `is_symbol`) and
  **display** (`to_display`, `print`).
- **Arithmetic** (`add`, `sub`, `mul`, variadic; `div` truncating-integer;
  `lt`, `gt`).
- **Closures** (`Closure`, `apply`, `make_closure`), an in-memory **global
  store** (`global_set`, `global_get`, `global_get_static`), and **builtin
  dispatch** (`call_builtin`, `builtin_closure`).

Migrated verbatim (behaviour-preserving) from the inline `_sir_*` prelude that
`semantic-ir-to-python` used to paste into every artifact. See
`code/specs/sir-runtime.md`.
