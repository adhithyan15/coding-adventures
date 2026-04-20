# logic-builtins

`logic-builtins` adds practical Prolog-inspired control, term inspection,
arithmetic, and collection predicates to the Python logic stack.

These functions are library goals, not syntax. They compose with
`logic-engine`, `logic-stdlib`, and the VM/bytecode layers because they return
ordinary logic goal expressions.

## What It Adds

- `callo(goal)`
- `onceo(goal)`
- `noto(goal)` for negation as failure
- `trueo()` and `failo()`
- `iftheno(condition, then_goal)` and `ifthenelseo(condition, then_goal, else_goal)`
- `forallo(generator, test)`
- `groundo(term)`
- `varo(term)` and `nonvaro(term)`
- `atomo(term)`, `numbero(term)`, `stringo(term)`, and `compoundo(term)`
- `functoro(term, name, arity)`
- `argo(index, term, value)`
- arithmetic expression constructors: `add`, `sub`, `mul`, `div`, `floordiv`, `mod`, and `neg`
- `iso(result, expression)` for Prolog-style evaluative arithmetic
- `numeqo(left, right)`, `numneqo(left, right)`, `lto(left, right)`, `leqo(left, right)`, `gto(left, right)`, and `geqo(left, right)`
- `findallo(template, goal, results)`, `bagofo(template, goal, results)`, and `setofo(template, goal, results)`

## Quick Start

```python
from logic_builtins import (
    add,
    argo,
    findallo,
    forallo,
    functoro,
    geqo,
    groundo,
    ifthenelseo,
    iso,
    noto,
    onceo,
)
from logic_engine import atom, conj, eq, fail, logic_list, num, program, solve_all, term, var

X = var("X")
Name = var("Name")
Arity = var("Arity")
Arg = var("Arg")
Score = var("Score")
Results = var("Results")

assert solve_all(program(), X, onceo(eq(X, "first"))) == [atom("first")]
assert solve_all(program(), X, noto(fail())) == [X]
assert solve_all(program(), X, conj(eq(X, term("box", "tea")), groundo(X))) == [
    term("box", "tea"),
]
assert solve_all(
    program(),
    (Name, Arity, Arg),
    conj(
        functoro(term("box", "tea"), Name, Arity),
        argo(1, term("box", "tea"), Arg),
    ),
) == [(atom("box"), num(1), atom("tea"))]
assert solve_all(program(), Score, iso(Score, add(40, 2))) == [num(42)]
assert solve_all(program(), X, conj(eq(X, 7), geqo(add(X, 1), 8))) == [num(7)]
assert solve_all(
    program(),
    Results,
    findallo(X, conj(eq(X, 7), geqo(add(X, 1), 8)), Results),
) == [logic_list([7])]
assert solve_all(
    program(),
    X,
    ifthenelseo(eq(X, "tea"), eq(X, "tea"), eq(X, "coffee")),
) == [atom("tea")]
assert solve_all(
    program(),
    X,
    forallo(conj(eq(X, 7)), geqo(X, 7)),
) == [X]
```

Arithmetic is evaluative, not a constraint system yet. `iso(Y, add(X, 1))`
fails while `X` is unbound, and succeeds after a goal such as `eq(X, 4)` has
instantiated it.

Collections are observations over a nested proof search. `findallo` succeeds
with an empty list when the inner goal fails, while `bagofo` and `setofo` fail
for empty collections.

Advanced control is intentionally honest about the current solver. `iftheno`
and `ifthenelseo` commit to the first condition proof while allowing the chosen
branch to keep backtracking. `forallo` checks every generated proof without
leaking generator bindings to the outer query. Full Prolog cut is not included
yet because it needs solver-level choicepoint pruning rather than a simple
library predicate.

## Dependencies

- logic-engine

## Development

```bash
bash BUILD
```
