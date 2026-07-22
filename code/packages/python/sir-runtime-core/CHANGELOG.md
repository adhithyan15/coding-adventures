# Changelog

## 0.2.0 — comparison helpers: `ne`, `le`, `ge`

Adds the runtime helpers the SIR backends need to lower Ruby's `!=`, `<=`, `>=`
(previously only `eq`/`lt`/`gt` existed, so those operators had nowhere to go).

- `ne(a, b)` — the exact negation of `eq`, inheriting its symbol-awareness
  (`:x != :x` is false), so `==` and `!=` never disagree.
- `le(a, b)` / `ge(a, b)` — native `<=` / `>=`, so `1 <= 1.0` is true (int and
  float compare by value, as in Ruby).

All three are exported and registered in the `call_builtin` dispatch table
alongside `==` (a synonym for `=`), so a first-class `:==`/`:!=` symbol
reference dispatches and the "known builtins" error message lists them.
All notable changes to `coding-adventures-sir-runtime-core` are documented here.

## [0.1.10] - 2026-07-07

### Fixed — division now floors (Ruby `Integer#/`) and true-divides floats (`Float#/`), SIR21 §E3

`div` used a single `int(a / b)`, which got Ruby's polymorphic `/` wrong two ways:

- **Integer division truncated toward zero** instead of flooring toward −∞ —
  `div(-7, 2)` gave `-3`, but Ruby (and the Rust oracle's
  [`DivOp::Floor`](../../rust/sir-conformance/src/oracle.rs)) say `-4`.
- **Float division was silently floored to an `int`** — `div(7.0, 2)` gave `3`,
  but Ruby's `Float#/` true-divides to `3.5`. A latent bug the truncating form
  masked, since the corpus exercised no float division.

`div` now dispatches on operand type (explicit `isinstance`, never reflection —
matching the `add`/`mul` style): two ints floor via Python's `//` (which also
rounds toward −∞, so it matches the oracle exactly on every sign combination);
anything involving a float true-divides via `/`. `bool` is excluded from the
integer path. The typed division-by-zero (T1) behaviour is unchanged.

This closes the **Python arm** of the division frontier captured in
`sir-conformance`'s `tests/division.rs`; `python_division_is_ruby_floor_faithful`
there is the non-ignored end-to-end regression guard. (JavaScript, Go and Rust
remain to be made floor-faithful.)

## [0.1.9] - 2026-07-07

### Added — source-language display convention (SIR display-convention spec)

- `set_display_convention(name)` selects the value-display convention:
  `"ruby"` renders booleans as `true`/`false`; any other name (the default)
  keeps the Lisp `#t`/`#f`. `to_display` now branches its boolean arm on the
  active convention. Module-level state — an emitted Ruby program calls the
  setter once at startup (each program is its own process). The default is
  unchanged, so all existing behaviour and callers are byte-for-byte identical
  until the setter is invoked. Mirrors the Rust/Go/JS backends' boolean
  display-convention increment. `nil`, symbol, and pair forms are unaffected
  (convention-independent for now; follow-ups per the spec rollout).

## [0.1.8] - 2026-07-01

### Added — typed division-by-zero (T1 of sir-typed-runtime-errors)

Ruby's `1 / 0` (and, per the SIR error spec, `1.0 / 0`) raises
`ZeroDivisionError`. Python's native `/` also raises on a zero divisor, but as a
**native** `ZeroDivisionError` — which the SIR rescue matcher only sees as an
over-broad `StandardError`, so a Ruby `rescue ZeroDivisionError` would miss it.
See
[`code/specs/sir-typed-runtime-errors.md`](../../../specs/sir-typed-runtime-errors.md).

- **`div`** now catches the native `ZeroDivisionError` and re-raises it as a
  **typed** `SirError` (`sir_class == "ZeroDivisionError"`, message
  `"divided by 0"`) via the shared `raise_error` entry point from
  `coding-adventures-sir-runtime-exceptions`. So `begin; 1 / 0; rescue
  ZeroDivisionError => e; end` now catches it, and `rescue StandardError` / a bare
  `rescue` still catch it via the ancestry walk. Applies per fold-step, so a
  variadic `div(a, b, 0)` reports the zero divisor it actually hit.
- No reflection or `eval`: the typed raise is an explicit class-name string,
  identical in shape to the `raise ZeroDivisionError` the frontend already emits.

### Dependency

- Added `coding-adventures-sir-runtime-exceptions` (a leaf package, no cycle) so
  `div` can reach the typed-raise entry point.

## [0.1.7] - 2026-07-01

### Added — polymorphic `+` / `*` for strings and arrays (PO1 of sir-polymorphic-operators)

Ruby's `+` and `*` are polymorphic on the receiver type, but the runtime only
implemented the **numeric** case. `mul` now dispatches explicitly on the runtime
tag (`isinstance`, never reflection) to match Ruby exactly, and `add` is fixed to
concatenate strings/arrays as well as sum numbers. See
[`code/specs/sir-polymorphic-operators.md`](../../../specs/sir-polymorphic-operators.md).

- **`add` (`+`)** now handles all three Ruby arms:
  - `add(1, 2, 3) == 6` (numeric, unchanged), `add() == 0` (identity, unchanged).
  - `add("a", "b") == "ab"` and `add("foo", "bar", "baz") == "foobarbaz"` (String
    concat).
  - `add([1], [2]) == [1, 2]` (Array concat, a **fresh** list — operands are not
    mutated, matching Ruby's non-destructive `Array#+`).
  - Fix: the fold is now **seeded with the first operand** instead of the integer
    `0`. The previous `total = 0; total += a` raised `TypeError` on the very first
    string/array operand (`0 + "a"`), so string/array `+` was silently broken;
    seeding from `args[0]` starts the fold in the right type. Numeric results are
    identical (addition is associative).
- **`mul` (`*`)** now implements Ruby's four arms, dispatched on the two-operand
  (binary) shape the frontend lowers, with the variadic numeric fold preserved for
  the pure-numeric case:
  - `str × int` → repeated string (`mul("ab", 3) == "ababab"`; count `<= 0` → `""`).
  - `list × int` → repeated-element list (`mul([0], 3) == [0, 0, 0]`; fresh list).
  - `list × str` → join elements with the separator, using the canonical
    `to_display` (element `to_s`, not `repr`): `mul([1, 2], ", ") == "1, 2"`,
    `mul([None, True], "|") == "nil|#t"`.
  - Otherwise the numeric fold, unchanged: `mul() == 1`, `mul(2, 3, 4) == 24`.
  - `bool` is excluded from the integer repeat/join arms (it is an `int` subclass
    in Python but not a SIR number, per `values.is_number`), so a bool count is
    never treated as a repeat count.
- No core-IR or frontend change — this is a runtime-only correctness fix on the
  Python backend. New regression tests cover every arm plus numeric preservation
  and non-aliasing (`test_add_string_concat`, `test_add_array_concat`,
  `test_add_array_concat_does_not_alias_operands`, `test_mul_string_repeat`,
  `test_mul_array_repeat`, `test_mul_array_repeat_does_not_alias_operand`,
  `test_mul_array_join_with_string`, `test_mul_numeric_fold_unchanged`,
  `test_mul_bool_count_is_not_treated_as_a_repeat`).
- Bumps `pyproject.toml` to `0.1.7`.

## [0.1.6] - 2026-07-01

### Security — cycle-guard the `puts` array flatten (CWE-674)

- `_puts_one` flattened arrays by recursing per element with **no bound**. A
  list is a shared, mutable reference, so a translated program can build a
  self-referential array (`a = []; a << a; puts a`) or a pathologically deep
  one; the unguarded recursion raised `RecursionError` — a denial of service
  (uncontrolled recursion). `_puts_one` / `sir_puts` now thread a `seen` set of
  `id(list)` on the active flatten path: a list re-encountered within its own
  subtree is a cycle and is printed as Ruby's `[...]` placeholder + newline
  instead of recursing, so `puts a` on a self-referential array now
  **terminates** exactly as real Ruby does. Non-cyclic output is unchanged
  (`puts [1, [2, 3]]` → `1\n2\n3\n`); new regression tests
  (`test_puts_self_referential_array_terminates`,
  `test_puts_mutually_recursive_arrays_terminate`) cover the cyclic cases.
- Bumps `pyproject.toml` to `0.1.6`.

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
