# logic-engine

`logic-engine` is the first Prolog-ready solving layer built on top of
[`logic-core`](../logic-core/).

It keeps LP00's terms, substitutions, and unification, then adds the pieces
that make relational programs feel like programs:

- named relations such as `parent/2`
- facts and rules
- immutable clause databases
- recursive solving with depth-first backtracking
- end-to-end query helpers like `solve_all()` and `solve_n()`

## Quick Start

```python
from logic_engine import atom, conj, fact, fresh, program, relation, rule, solve_all, var

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")

family = program(
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(
        ancestor(X, Y),
        fresh(1, lambda z: conj(parent(X, z), ancestor(z, Y))),
    ),
)

assert solve_all(family, Y, ancestor("homer", Y)) == [atom("bart"), atom("lisa")]
```

## Dependencies

- [`logic-core`](../logic-core/)

## Development

```bash
bash BUILD
```
