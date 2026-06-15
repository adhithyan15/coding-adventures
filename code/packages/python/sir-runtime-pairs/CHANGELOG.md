# Changelog

## 0.1.0 — initial release

First release of the SIR cons-pair runtime for Python, per
`code/specs/sir-runtime.md`. Extracts the `Pair` value type (and its
`cons`/`car`/`cdr` operators) into a dedicated per-concern runtime so the
display can be injected rather than imported, avoiding a load-time cycle with
`coding-adventures-sir-runtime-core`. Python mirror of
`@coding-adventures/sir-runtime-pairs` (TypeScript).

### Added

- `class Pair` — an immutable cons cell with `car` / `cdr` fields
  (`__slots__`); `__repr__` renders the Lisp list display (proper list
  `(1 2 3)`, dotted pair `(1 . 2)`) via the injectable display hook.
- `cons(a, b) -> Pair`, `car(p)`, `cdr(p)` — construct/access; `car`/`cdr`
  raise `TypeError` on a non-pair.
- `is_pair(v) -> bool` — pair predicate.
- `set_display(fn)` — inject the element renderer; defaults to `str`. Core
  injects its richer `to_display` here so pairs render with full SIR display
  while this package keeps **zero dependencies** and never imports core.
- `Val` — the universal SIR value type alias at this boundary.
- Full standard layout: `pyproject.toml` (src layout), `BUILD`, `BUILD_windows`,
  `required_capabilities.json` (no capabilities), `py.typed`, README. pytest
  suite at 100% coverage of `pairs.py`; `mypy --strict` + `ruff` clean.

### Design note

The display is a module-level **hook** (a real default function, not a lambda,
to satisfy ruff E731) rather than an import of core's `to_display`. This inverts
the pairs↔core dependency so neither imports the other at module-load time;
core injects its display via `set_display` at import.
