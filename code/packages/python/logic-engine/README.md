# logic-engine

`logic-engine` is the first Prolog-ready solving layer built on top of
[`logic-core`](../logic-core/).

It keeps LP00's terms, substitutions, and unification, then adds the pieces
that make relational programs feel like programs:

- named relations such as `parent/2`
- facts and rules
- immutable clause databases
- recursive solving with depth-first backtracking
- deferred recursive goal builders via `defer(...)`
- disequality constraints via `neq(...)`
- `all_different(...)` for small puzzle-style searches
- end-to-end query helpers like `solve_all()` and `solve_n()`

## Quick Start

```python
from logic_engine import all_different, atom, conj, fact, neq, program, relation, solve_n, var

color = relation("color", 1)

WA = var("WA")
NT = var("NT")
SA = var("SA")

palette = program(
    fact(color("red")),
    fact(color("green")),
    fact(color("blue")),
)

answers = solve_n(
    palette,
    3,
    (WA, NT, SA),
    conj(
        color(WA),
        color(NT),
        color(SA),
        all_different(WA, NT, SA),
        neq(WA, atom("blue")),
    ),
)

assert answers
```

## Dependencies

- [`logic-core`](../logic-core/)

## Development

```bash
bash BUILD
```
