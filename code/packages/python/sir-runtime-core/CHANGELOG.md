# Changelog

All notable changes to `coding-adventures-sir-runtime-core` are documented here.

## [0.1.5] - 2026-07-01

### Added (`puts` — Ruby's most common output method)

- New `sir_puts(*args)` implementing Ruby `puts` semantics, and a `"puts"`
  entry in the builtin dispatch table (so backends that route builtins by name
  reach it). Exposed from the package root and re-exported in `__all__`.
- Semantics, matching Ruby exactly:
  - `puts` (no args) → a single newline.
  - `puts x` → `x.to_s` + newline, **unless** the rendered text already ends in
    `"\n"` (then no second newline: `puts "x\n"` prints `x\n`, not `x\n\n`).
  - `puts a, b` → each argument on its own line, in order.
  - `puts []` → a single newline (an argument that flattens to nothing still
    prints a blank line).
  - `puts [1, [2, 3]]` → each **element** on its own line, arrays flattened
    recursively (`1\n2\n3\n`).
  - `puts nil` → a blank line (not the display form `"nil"`).
- Also bumps `pyproject.toml` to `0.1.5` (it had lagged the changelog at
  `0.1.2`).

## [0.1.4] - 2026-06-21

### Added (Q10g — proc-vs-lambda arity)

- `Closure` now carries `arity` (fixed positional params after captures, or
  `None` if variadic) and `is_lambda`. `make_closure` introspects the wrapped
  function to record the block's arity; `apply` uses it to give **proc/block
  leniency** — extra arguments dropped, missing ones become `nil` (`None`),
  matching Ruby (e.g. a one-param block yielded two values binds the first).
- New `as_lambda(c)` marks a closure **strict** and returns it. The `lambda` /
  `->(){}` builtin wraps its closure with it so a lambda's arity mismatch
  raises (the analogue of Ruby's `ArgumentError`) instead of being adjusted.
- A variadic block (`|*rest|`, `arity is None`) is passed through unadjusted.
  v0 cut-line: optional/keyword block params are counted as required positions
  (see `code/specs/sir-runtime.md`).

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
