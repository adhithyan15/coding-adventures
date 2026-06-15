# coding-adventures-sir-runtime-pairs

Cons-pair runtime for **Semantic-IR-emitted Python**.

The SIR (Semantic IR) backends translate most Ruby-surface constructs to
*native* Python: a sequence becomes a `list`, a map becomes a `dict`. The Lisp
**cons cell** has no native Python equivalent — a `tuple` is variadic and has no
list *display* — so the SIR `Pair` value type and its `cons` / `car` / `cdr`
operators live here.

A *pair* is an immutable two-field record holding a `car` (first) and `cdr`
(rest). Linked pairs build lists. A proper list `(1 2 3)` is
`cons(1, cons(2, cons(3, None)))` with `None` for `nil`; an improper (dotted)
pair `(1 . 2)` is `cons(1, 2)`.

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-python ─▶ .py
                                                                             │ imports
                                                                             ▼
                                                       coding-adventures-sir-runtime-pairs
```

The Python backend emits an import of this package only when a module uses
pairs; pure modules never gain the dependency.

## The extraction + injection design (no cycle with core)

The general SIR value display lives in
[`coding-adventures-sir-runtime-core`](../sir-runtime-core) as `to_display`. A
pair wants to render its elements with that richer display (so a boolean inside
a list prints as `#t`/`#f` rather than `True`/`False`). But core *also* needs to
display pairs — a pair nested inside some other value — so the two importing
each other would form a load-time cycle.

We break the cycle by **inverting the dependency**: this package depends on
**nothing** and exposes a module-level display *hook*, defaulting to `str`. When
core is present it calls `set_display(to_display)` once at import time, and from
then on pairs render as proper Lisp lists. Used standalone, a pair still prints
sensibly — just with `str` for each element. **Pairs never import core.**

```text
pairs ◀───── set_display(to_display) ───── core   (core knows pairs;
  │                                                 pairs never imports
  └─ depends on nothing ────────────────────────── core)
```

## API

| Export | Purpose |
|---|---|
| `class Pair` | Immutable cons cell with `car` / `cdr` (`__slots__`); `__repr__` is the Lisp list display via the injected hook. |
| `cons(a, b) -> Pair` | Construct the pair `(a . b)`. |
| `car(p) -> Val` | First field; raises `TypeError("car on non-pair")` on a non-pair. |
| `cdr(p) -> Val` | Rest field; raises `TypeError("cdr on non-pair")` on a non-pair. |
| `is_pair(v) -> bool` | True iff `v` is a `Pair`. |
| `set_display(fn) -> None` | Inject the element renderer (core does this with `to_display`). |
| `Val` | The universal SIR value type alias (`Any`) at this boundary. |

## Usage

```python
from coding_adventures_sir_runtime_pairs import cons, car, cdr, is_pair

p = cons(1, cons(2, cons(3, None)))   # the proper list (1 2 3)
car(p)                                # 1
cdr(p)                                # (2 3)
is_pair(p)                            # True
repr(p)                               # "(1 2 3)"
repr(cons(1, 2))                      # "(1 . 2)"  (dotted pair)
```

Injecting a richer display (what core does):

```python
from coding_adventures_sir_runtime_pairs import cons, set_display

set_display(lambda v: "nil" if v is None else str(v))
repr(cons(1, None))   # "(1 . nil)"
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
