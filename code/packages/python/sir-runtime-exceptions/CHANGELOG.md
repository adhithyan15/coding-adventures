# Changelog

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
