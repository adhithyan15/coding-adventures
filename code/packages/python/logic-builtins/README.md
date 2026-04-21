# logic-builtins

`logic-builtins` adds practical Prolog-inspired control, term inspection,
arithmetic, and collection predicates to the Python logic stack.

These functions are library goals, not syntax. They compose with
`logic-engine`, `logic-stdlib`, and the VM/bytecode layers because they return
ordinary logic goal expressions.

## What It Adds

- `callo(goal)`
- `calltermo(term_goal)` for executing reified callable goal terms
- `onceo(goal)`
- `noto(goal)` for negation as failure
- `trueo()` and `failo()`
- `iftheno(condition, then_goal)` and `ifthenelseo(condition, then_goal, else_goal)`
- `forallo(generator, test)`
- `groundo(term)`
- `varo(term)` and `nonvaro(term)`
- `atomo(term)`, `numbero(term)`, `stringo(term)`, and `compoundo(term)`
- `atomico(term)` and `callableo(term)`
- `functoro(term, name, arity)` for inspection and construction
- `argo(index, term, value)`
- `univo(term, parts)` for Prolog-style `=../2` term decomposition/construction
- `copytermo(source, copy)` and `same_termo(left, right)`
- `clauseo(head, body)` for Prolog-style clause introspection
- `compare_termo(order, left, right)`, `termo_lto(left, right)`,
  `termo_leqo(left, right)`, `termo_gto(left, right)`, and
  `termo_geqo(left, right)` for standard term ordering
- `current_predicateo(name, arity)` and
  `predicate_propertyo(name, arity, property)` for predicate metadata
- `dynamico(name, arity)`, `assertao(clause)`, `assertzo(clause)`,
  `retracto(clause)`, `retractallo(head)`, and `abolisho(name, arity)` for
  branch-local dynamic database mutation
- arithmetic expression constructors: `add`, `sub`, `mul`, `div`, `floordiv`, `mod`, and `neg`
- `iso(result, expression)` for Prolog-style evaluative arithmetic
- `numeqo(left, right)`, `numneqo(left, right)`, `lto(left, right)`, `leqo(left, right)`, `gto(left, right)`, and `geqo(left, right)`
- `findallo(template, goal, results)`, `bagofo(template, goal, results)`, and `setofo(template, goal, results)`

## Quick Start

```python
from logic_builtins import (
    add,
    assertzo,
    argo,
    dynamico,
    calltermo,
    clauseo,
    compare_termo,
    current_predicateo,
    findallo,
    forallo,
    functoro,
    geqo,
    groundo,
    ifthenelseo,
    iso,
    noto,
    onceo,
    predicate_propertyo,
    same_termo,
    termo_lto,
    univo,
)
from logic_engine import (
    atom,
    conj,
    eq,
    fail,
    logic_list,
    num,
    program,
    relation,
    rule,
    solve_all,
    term,
    var,
)

X = var("X")
Name = var("Name")
Arity = var("Arity")
Arg = var("Arg")
Score = var("Score")
Results = var("Results")
Body = var("Body")
Order = var("Order")
Property = var("Property")

parent = relation("parent", 2)
child = relation("child", 2)
memo = relation("memo", 1)
family = program(rule(child(X, Name), parent(Name, X)))

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
assert solve_all(
    program(),
    Results,
    univo(term("box", "tea", "cake"), Results),
) == [logic_list(["box", "tea", "cake"])]
assert solve_all(
    program(),
    X,
    univo(X, logic_list(["box", "tea", "cake"])),
) == [term("box", "tea", "cake")]
assert solve_all(program(), X, same_termo(X, X)) == [X]
assert solve_all(family, Body, clauseo(child("bart", "homer"), Body)) == [
    term("parent", "homer", "bart"),
]
assert solve_all(
    family,
    Body,
    conj(clauseo(child("bart", "homer"), Body), calltermo(Body)),
) == [term("parent", "homer", "bart")]
assert solve_all(program(), Order, compare_termo(Order, X, 7)) == [atom("<")]
assert solve_all(program(), X, conj(eq(X, "ok"), termo_lto(X, term("box", "tea")))) == [
    atom("ok"),
]
assert solve_all(family, Arity, current_predicateo("parent", Arity)) == [num(2)]
assert term("number_of_clauses", 1) in solve_all(
    family,
    Property,
    predicate_propertyo("child", 2, Property),
)
assert solve_all(
    program(),
    X,
    conj(dynamico("memo", 1), assertzo(memo("cached")), memo(X)),
) == [atom("cached")]
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

Term metaprogramming treats terms as ordinary data. `univo` decomposes
`box(tea, cake)` into `[box, tea, cake]` and can construct the term back from
that list. `functoro` now constructs atoms and compounds when supplied a name
and arity. `copytermo` refreshes variables in a copied term, while
`same_termo` checks strict identity without binding variables.

Clause introspection treats source clauses as ordinary data. `clauseo(Head,
Body)` enumerates facts with body `true` and rules with a term-encoded body,
standardizing variables apart before unifying with the query.

Callable term execution closes that loop. `calltermo(Body)` can execute a body
returned by `clauseo`, so metaprograms can inspect source clauses and then run
the represented goals. Standard term-order predicates compare reified terms
without binding them, while predicate metadata exposes source predicates and
the builtin predicate surface as ordinary logic-queryable facts.

Dynamic database builtins are branch-local. A clause asserted by `assertao` or
`assertzo` is visible to later goals reached from that state, while normal
backtracking restores the previous database snapshot. Static source predicates
must be declared dynamic at the program level before runtime retraction or
abolition can affect their clauses.

## Dependencies

- logic-engine

## Development

```bash
bash BUILD
```
