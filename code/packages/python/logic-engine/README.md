# logic-engine

`logic-engine` is the first Prolog-ready solving layer built on top of
[`logic-core`](../logic-core/).

It keeps LP00's terms, substitutions, and unification, then adds the pieces
that make relational programs feel like programs:

- named relations such as `parent/2`
- facts and rules
- immutable clause databases
- first-order clause and goal term encoders
- callable goal-term lowering via `goal_from_term(...)`
- persistent database updates via `asserta(...)`, `assertz(...)`, `retract_*`,
  and `abolish(...)`
- recursive solving with depth-first backtracking
- deferred recursive goal builders via `defer(...)`
- state-aware native goal hooks via `native_goal(...)` for library builtins
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

## Persistent Clause Databases

Database helpers return new programs instead of mutating the existing program.
That gives Python callers Prolog-style database operations while keeping
backtracking deterministic.

```python
from logic_engine import atom, assertz, fact, program, relation, retract_first, solve_all, var

parent = relation("parent", 2)
X = var("X")

family = program(fact(parent("homer", "bart")))
expanded = assertz(family, fact(parent("homer", "lisa")))

assert solve_all(family, X, parent("homer", X)) == [atom("bart")]
assert solve_all(expanded, X, parent("homer", X)) == [atom("bart"), atom("lisa")]

without_first = retract_first(expanded, parent("homer", X))
assert without_first is not None
assert solve_all(without_first, X, parent("homer", X)) == [atom("lisa")]
```

## Clause Introspection

Clauses can also be viewed as ordinary term data. Facts use body `true`, while
rules expose their goal body.

```python
from logic_engine import clause_as_term, fact, relation, rule, term, var

parent = relation("parent", 2)
child = relation("child", 2)
X = var("X")
Y = var("Y")

assert clause_as_term(fact(parent("homer", "bart"))) == term(
    ":-",
    term("parent", "homer", "bart"),
    "true",
)
assert clause_as_term(rule(child(X, Y), parent(Y, X))) == term(
    ":-",
    term("child", X, Y),
    term("parent", Y, X),
)
```

## Callable Goal Terms

`goal_from_term(...)` turns Prolog-shaped goal data back into an executable
engine goal. This is the core bridge that lets higher layers inspect a clause
body as data and then run that body again.

```python
from logic_engine import atom, fact, goal_from_term, program, relation, solve_all, term, var

parent = relation("parent", 2)
Child = var("Child")

family = program(
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
)

assert solve_all(family, Child, goal_from_term(term("parent", "homer", Child))) == [
    atom("bart"),
    atom("lisa"),
]
```

## Dependencies

- [`logic-core`](../logic-core/)

## Development

```bash
bash BUILD
```
