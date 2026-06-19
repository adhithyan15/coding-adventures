# Changelog

All notable changes to `coding-adventures-sir-runtime-core` are documented here.

## [0.1.3] - 2026-06-19

### Added (Q10a — no-block-given `LocalJumpError`)

- New `LocalJumpError` exception. `apply(None, args)` now raises it
  ("no block given (yield)") instead of a generic `TypeError`. This is
  the SIR analogue of Ruby's `LocalJumpError`: under the explicit
  block-param ABI a method that `yield`s through a block parameter the
  caller never supplied reaches `apply` with a `None` target, and that
  failure is now distinct and recognisable (a genuine non-closure, e.g.
  applying an int, still raises `TypeError`). Exported from the package
  root. Ruby's exact class identity is not modelled — the analogue is
  keyed to the error's shape, not Ruby's hierarchy.

## [0.1.2] - 2026-06-15

### Changed

- `call_builtin` now raises a descriptive error for an unregistered SIR builtin
  — it names the builtin, lists the known ones, and explains it indicates a
  backend coverage gap (the emitter produced a `call_builtin` for something it
  does not lower natively or via a per-concern runtime package), rather than a
  bare `unknown builtin: <name>`.

## [0.1.1] - 2026-06-15

### Changed

- The cons-pair value type (`Pair` / `cons` / `car` / `cdr` / `is_pair`) has
  been **extracted** into the dedicated `coding-adventures-sir-runtime-pairs`
  package. `core.pairs` is now a thin re-export shim, so every existing import
  (`from coding_adventures_sir_runtime_core import cons`) and the builtin
  dispatch table keep working unchanged, and a pair built via core is the *same*
  class as one built via the dedicated package.
- Core now **depends on** `coding-adventures-sir-runtime-pairs` (resolved via a
  local `tool.uv.sources` path) and, at import time, injects its richer
  `to_display` into the (dependency-free) pairs package's display hook
  (`set_display`) so a `Pair` still renders as a Lisp list (`(1 2 3)`,
  `(1 . 2)`). This keeps the package dependency graph acyclic.

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
