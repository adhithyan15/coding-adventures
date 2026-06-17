# coding-adventures-sir-runtime-range

Range runtime for **Semantic-IR-emitted Python**.

The SIR backends translate most Ruby-surface constructs to *native* Python: a
sequence becomes a `list`, a map becomes a `dict`. A Ruby **range** is a
first-class object — you iterate it, test membership (`r.include?(3)`), or
materialise it (`r.to_a`) — and Python's `range` cannot stand in for it: it is
half-open only (no inclusive `1..5`), integer-stride only, and has no
begin/endless forms. So the SIR `Range` value type lives here, exactly like the
cons cell lives in [`sir-runtime-pairs`](../sir-runtime-pairs).

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                             │ imports
                                                                             ▼
                                                       coding-adventures-sir-runtime-range
```

A Ruby range literal lowers to `BuiltinCall("range", [start, stop, exclusive])`;
the Python backend emits `_sir_range(start, stop, exclusive)` and imports this
package **only** when a module actually uses a range — pure modules never gain
the dependency.

## The range forms

| Ruby | `start` | `stop` | `exclusive` | members |
|---|---|---|---|---|
| `1..5`  | `1`    | `5`    | `False` | `1 2 3 4 5` |
| `1...5` | `1`    | `5`    | `True`  | `1 2 3 4` |
| `1..`   | `1`    | `None` | `False` | `1 2 3 …` (endless) |
| `..5`   | `None` | `5`    | `False` | `… 4 5` (beginless) |

Iteration walks integers upward from `start`. An **endless** range yields
forever — consume it lazily (`next`, `itertools.islice`). A **beginless** range
has no first element, so iterating one (or calling `to_list` on any unbounded
range) raises `TypeError` rather than hanging — matching Ruby, where
`(..5).each` is a `TypeError`.

## API

| Export | Purpose |
|---|---|
| `class Range` | Immutable range with `start` / `stop` / `exclusive`; iterable, `in`-testable, value-equal, Ruby-notation `repr`. |
| `range(start, stop, exclusive) -> Range` | Construct a range (the backend's `_sir_range`). |
| `includes(r, v) -> bool` | Membership (Ruby `include?`). |
| `to_list(r) -> list` | Materialise (Ruby `to_a`); raises on an unbounded range. |
| `is_range(v) -> bool` | True iff `v` is a `Range`. |
| `Val` | The universal SIR value type alias (`Any`) at this boundary. |

## Usage

```python
from coding_adventures_sir_runtime_range import range as sir_range, to_list

r = sir_range(1, 5, False)   # the inclusive range 1..5
list(r)                      # [1, 2, 3, 4, 5]
3 in r                       # True
to_list(sir_range(1, 5, True))   # [1, 2, 3, 4]  (exclusive)
repr(sir_range(1, None, False))  # "1.."         (endless)
```

## Development

```bash
uv venv && uv pip install -e .[dev]
.venv/bin/python -m ruff check src tests
.venv/bin/python -m mypy
.venv/bin/python -m pytest tests/ -v
```

## License

MIT
