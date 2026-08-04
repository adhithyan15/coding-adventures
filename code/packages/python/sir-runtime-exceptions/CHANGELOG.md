# Changelog

## 0.3.0 — `ancestry_chain` accessor for module-aware `is_a?`

Exposes `ancestry_chain(class_name)`: the exception's class followed by each of
its registered ancestors, in order (`"ArgumentError"` →
`["ArgumentError", "StandardError", "Exception"]`). Cycle-safe — a malformed
self- or mutual edge terminates, and each class appears at most once.

The ancestry table stays private (`_ANCESTRY`); this is a read-only *view* of
it. The OOP runtime's `is_a?` needs to visit every link to ask whether a
module was `include`d there — a per-link question that `rescue_matches` (a
boolean name-walk over the whole chain) cannot answer. Added because a
security review found `Exception#is_a?` on the sibling `sir-runtime-oop`
package ignored included modules where the Rust/Go/JavaScript backends honour
them, so `retry unless e.is_a?(Recoverable)` took the opposite branch on
Python alone.

## 0.2.0 — user-defined exception-class ancestry (E2)

Adds `register_ancestry(mapping)`: the SIR backend threads user
`class Child < Parent` edges here at program init (an explicit
`{childClassName: superclassName}` string map — no `eval`/reflection), so
`rescue_matches` walks a user class up through its registered superclass and on
into the built-in table. A `rescue StandardError` now catches a raised user
`MyErr < StandardError`.

- User edges are **additive**: they layer over the frozen built-in ancestry
  without replacing it, so built-in matching is unchanged. A user class with no
  registered edge still matches only by exact name (or via `Exception` / a bare
  `rescue`).
- The cycle guard in `_is_ancestor_or_self` already tolerates a malformed
  self-referential edge.
- `register_ancestry` is re-exported from the package root.

## 0.1.0 — initial release

First release of the SIR exception runtime for Python, per
`code/specs/sir-runtime.md`. Supplies the two pieces of Ruby-surface exception
handling that have no faithful native equivalent, so the rest can translate to a
native `try / except / finally`. Python mirror of
`@coding-adventures/sir-runtime-exceptions` (TypeScript).

### Added

- `class SirError(Exception)` — the raised object, a real `Exception` carrying
  the Ruby/SIR class name in `sir_class` (with `__slots__`); the message
  defaults to the class name.
- `raise_error(class_name="RuntimeError", message=None) -> NoReturn` — raises a
  `SirError`; bare `raise_error()` re-raises as a generic `RuntimeError`.
- `class_of_thrown(exc)` — the SIR class name of a caught value (native Python
  errors bucket as `StandardError`).
- `rescue_matches(exc, class_names)` — rescue-clause class matching against a
  curated built-in Ruby ancestry table; empty list = bare `rescue` (catch-all),
  `Exception` = universal root, user classes match by exact name. Cycle-safe
  ancestry walk.
- Full standard layout: `pyproject.toml` (src layout), `BUILD`, `BUILD_windows`,
  `required_capabilities.json` (no capabilities), `py.typed`, README. 16 pytest
  cases; `mypy --strict` + `ruff` clean.

### v0 limitation (documented)

User-defined exception-class ancestry is unknown (no SIR exception-class symbol
table), so `rescue StandardError` matches only the built-in subclasses, and user
classes match by exact name. A bare re-raise becomes a generic `RuntimeError`.
Both await a frontend that threads the exception model into SIR.
