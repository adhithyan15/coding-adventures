# Changelog

## 0.1.0 — initial release

First release of the SIR range runtime for Python, per
`code/specs/sir-runtime.md`. A Ruby range (`1..5`, `1...5`, `1..`, `..5`) is a
first-class value, not a loop; Python's `range` is half-open, integer-only, and
cannot model the inclusive or begin/endless forms, so the SIR `Range` ships here
as a dedicated per-concern runtime (mirroring `sir-runtime-pairs`). Python
mirror of `@coding-adventures/sir-runtime-range` (TypeScript).

### Added

- `class Range` — an immutable range value with `start` / `stop` / `exclusive`
  fields (`__slots__`). Iterable (`__iter__`), supports `in` (`__contains__`),
  value equality + hashing, and a Ruby-notation `__repr__` (`1..5` / `1...5` /
  `1..` / `..5`).
- `range(start, stop, exclusive) -> Range` — the constructor the Python backend
  targets (`_sir_range(...)`); re-exported under the SIR name `range`
  (internally `range_` to avoid shadowing the builtin).
- `includes(r, v)` / `Range.includes` — membership (Ruby `include?`), correct
  for inclusive, exclusive, beginless, and endless forms.
- `to_list(r)` / `Range.to_list` — materialise as a list (Ruby `to_a`); raises
  `TypeError` for an unbounded (beginless or endless) range.
- `is_range(v) -> bool` — range predicate. `Val` — the universal SIR value type
  alias at this boundary.
- Full standard layout: `pyproject.toml` (src layout), `BUILD`, `BUILD_windows`,
  `required_capabilities.json` (no capabilities), `py.typed`, README. pytest
  suite at 100% coverage of `range.py`; `mypy --strict` + `ruff` clean.

### Semantics (v0)

- Iteration walks integers upward from `start`. An endless range yields forever
  (consume lazily); a beginless range raises `TypeError` on iteration (it has no
  first element) — matching Ruby's behaviour for `(..5).each`.
- Membership and the begin/endless forms are fully supported; non-integer
  stride / float ranges are out of scope for v0.
