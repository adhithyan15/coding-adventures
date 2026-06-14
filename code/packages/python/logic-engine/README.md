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
- dynamic predicate declarations and branch-local runtime database overlays
- finite-domain store preservation for CLP(FD)-style builtins
- Prolog runtime flag overlay preservation for dialect/runtime builtins
- scoped search control via `cut()` as the library form of Prolog `!/0`
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

## Dynamic Runtime Databases

Dynamic predicates opt into branch-local mutation during solving. The runtime
database lives on `State`, so asserted clauses are visible to later goals in
the same proof branch and disappear automatically when search backtracks to an
older state.

```python
from logic_engine import (
    State,
    atom,
    fact,
    program,
    relation,
    runtime_assertz,
    runtime_declare_dynamic,
    solve_from,
    var,
)

edge = relation("edge", 2)
X = var("X")

state = runtime_declare_dynamic(program(), State(), edge)
assert state is not None
state = runtime_assertz(program(), state, fact(edge("a", "b")))
assert state is not None

answers = [
    answer.substitution.reify(X)
    for answer in solve_from(program(), edge("a", X), state)
]
assert answers == [atom("b")]
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

## Search Control

`cut()` succeeds once and commits to choices made earlier in the current query
or predicate invocation. Choices created after the cut can still backtrack, and
cuts inside a relation body are consumed by that relation call instead of
leaking into the caller.

```python
from logic_engine import atom, conj, cut, disj, eq, program, solve_all, var

X = var("X")

assert solve_all(
    program(),
    X,
    conj(disj(eq(X, "first"), eq(X, "second")), cut()),
) == [atom("first")]
assert solve_all(
    program(),
    X,
    conj(cut(), disj(eq(X, "first"), eq(X, "second"))),
) == [atom("first"), atom("second")]
```

## Dependencies

- [`logic-core`](../logic-core/)

## Development

```bash
bash BUILD
```
