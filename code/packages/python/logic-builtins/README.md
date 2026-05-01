# logic-builtins

`logic-builtins` adds practical Prolog-inspired control, finite-domain
constraints, term inspection, arithmetic, and collection predicates to the
Python logic stack.

These functions are library goals, not syntax. They compose with
`logic-engine`, `logic-stdlib`, and the VM/bytecode layers because they return
ordinary logic goal expressions.

## What It Adds

- `callo(goal)`
- `calltermo(term_goal, *extra_args)` for executing reified callable goal terms
  and Prolog-style `call/N` argument extension
- `maplisto(closure, list)`, `maplisto(closure, left, right)`,
  `maplisto(closure, first, second, third)`, `maplisto(closure, first, second,
  third, fourth)`, `convlisto(closure, items, results)`, `scanlo(closure,
  *lists_and_accumulators)`, `includeo(closure, items,
  included)`, `excludeo(closure, items, excluded)`, `partitiono(closure, items,
  included, excluded)`, and `foldlo(closure, *lists_and_accumulators)` for
  higher-order list processing through callable terms
- `onceo(goal)`
- `cuto()` as the library form of Prolog `!/0`
- `noto(goal)` for negation as failure
- `trueo()` and `failo()`
- `iftheno(condition, then_goal)` and `ifthenelseo(condition, then_goal, else_goal)`
- `forallo(generator, test)`
- `groundo(term)`, `acyclic_termo(term)`, and `cyclic_termo(term)` for
  term-shape checks
- `varo(term)` and `nonvaro(term)`
- `atomo(term)`, `integero(term)`, `numbero(term)`, `stringo(term)`, and
  `compoundo(term)`
- `atomico(term)` and `callableo(term)`
- `functoro(term, name, arity)` for inspection and construction
- `compound_name_argumentso(term, name, arguments)` and
  `compound_name_arityo(term, name, arity)` for compound-only reflection and
  construction
- `argo(index, term, value)`
- `univo(term, parts)` for Prolog-style `=../2` term decomposition/construction
- `unify_with_occurs_checko(left, right)` and `unifiableo(left, right, unifier)`
  for finite unification and non-binding unifiability inspection
- `copytermo(source, copy)`, `same_termo(left, right)`, and
  `not_same_termo(left, right)`
- `term_variableso(term, variables)` and `numbervarso(term, start, end)` for
  source-level term variable inspection and `'$VAR'(N)` numbering
- `term_hasho(term, hash)` and `term_hash_boundedo(term, depth, range, hash)`
  for stable structural term hashes
- `variant_termo(left, right)`, `not_variant_termo(left, right)`, and
  `subsumes_termo(general, specific)` for non-binding term generality checks
- `atom_charso/2`, `atom_codeso/2`, `number_charso/2`, `number_codeso/2`,
  `char_codeo/2`, `string_charso/2`, and `string_codeso/2` for finite text
  conversion relations
- `atom_concato/3`, `atomic_list_concato/2`,
  `atomic_list_concato_with_separator/3`, and `number_stringo/2` for finite
  atom composition and number/string conversion modes
- `atom_lengtho/2`, `string_lengtho/2`, `sub_atomo/5`, and `sub_stringo/5` for
  finite text inspection and slicing modes
- `difo(left, right)` for delayed disequality constraints
- `clauseo(head, body)` for Prolog-style clause introspection
- `compare_termo(order, left, right)`, `termo_lto(left, right)`,
  `termo_leqo(left, right)`, `termo_gto(left, right)`, and
  `termo_geqo(left, right)` for standard term ordering
- `current_predicateo(name, arity)` and
  `predicate_propertyo(name, arity, property)` for predicate metadata
- `current_prolog_flago(name, value)` and `set_prolog_flago(name, value)` for
  branch-local runtime flag metadata
- `dynamico(name, arity)`, `assertao(clause)`, `assertzo(clause)`,
  `retracto(clause)`, `retractallo(head)`, and `abolisho(name, arity)` for
  branch-local dynamic database mutation
- `fd_ino(var, domain)`, `fd_eqo(left, right)`, `fd_neqo(left, right)`,
  `fd_lto(left, right)`, `fd_leqo(left, right)`, `fd_gto(left, right)`,
  `fd_geqo(left, right)`, `fd_addo(left, right, result)`,
  `fd_subo(left, right, result)`, `fd_mulo(left, right, result)`,
  `fd_sumo(vars, total)`, `fd_scalar_producto(coeffs, vars, total)`,
  `fd_sum_relationo(vars, op, total)`, `fd_scalar_product_relationo(coeffs,
  vars, op, total)`, `fd_elemento(index, vars, value)`,
  `fd_reify_relationo(left, op, right, truth)`, `fd_bool_ando(left, right,
  result)`, `fd_bool_oro(left, right, result)`, `fd_bool_noto(value, result)`,
  `fd_bool_implieso(left, right, result)`, `fd_bool_equivo(left, right,
  result)`,
  `all_differento(vars)`, `labelingo(vars)`, and
  `labeling_optionso(options, vars)` for finite-domain integer constraints
- arithmetic expression constructors: `add`, `sub`, `mul`, `div`, `floordiv`, `mod`, and `neg`
- `iso(result, expression)` for Prolog-style evaluative arithmetic
- `numeqo(left, right)`, `numneqo(left, right)`, `lto(left, right)`, `leqo(left, right)`, `gto(left, right)`, and `geqo(left, right)`
- `betweeno(low, high, value)` for finite inclusive integer generation
- `succo(predecessor, successor)` for non-negative integer successor relations
- `findallo(template, goal, results)`, `bagofo(template, goal, results)`, and `setofo(template, goal, results)`

## Quick Start

```python
from logic_builtins import (
    add,
    assertzo,
    argo,
    betweeno,
    calltermo,
    clauseo,
    compare_termo,
    current_prolog_flago,
    current_predicateo,
    cuto,
    difo,
    dynamico,
    excludeo,
    all_differento,
    fd_addo,
    fd_ino,
    fd_leqo,
    fd_lto,
    fd_neqo,
    fd_sumo,
    findallo,
    foldlo,
    forallo,
    functoro,
    compound_name_argumentso,
    compound_name_arityo,
    geqo,
    groundo,
    acyclic_termo,
    cyclic_termo,
    unifiableo,
    unify_with_occurs_checko,
    ifthenelseo,
    includeo,
    integero,
    iso,
    labelingo,
    maplisto,
    atom_concato,
    atom_charso,
    atom_codeso,
    atom_lengtho,
    atomic_list_concato_with_separator,
    number_charso,
    number_stringo,
    char_codeo,
    not_same_termo,
    numbervarso,
    subsumes_termo,
    noto,
    onceo,
    partitiono,
    predicate_propertyo,
    same_termo,
    succo,
    set_prolog_flago,
    sub_atomo,
    sub_stringo,
    term_hash_boundedo,
    term_hasho,
    termo_lto,
    univo,
    variant_termo,
)
from logic_engine import (
    atom,
    conj,
    disj,
    eq,
    fail,
    fact,
    logic_list,
    num,
    program,
    relation,
    rule,
    solve_all,
    string,
    term,
    var,
)

X = var("X")
Y = var("Y")
Name = var("Name")
Arity = var("Arity")
Arg = var("Arg")
Score = var("Score")
Results = var("Results")
Body = var("Body")
Order = var("Order")
Property = var("Property")
FlagValue = var("FlagValue")

parent = relation("parent", 2)
child = relation("child", 2)
memo = relation("memo", 1)
family = program(rule(child(X, Name), parent(Name, X)))
increment = relation("increment", 2)
small = relation("small", 1)
push = relation("push", 3)
higher_order = program(
    fact(small(1)),
    fact(small(2)),
    fact(increment(1, 2)),
    fact(increment(2, 3)),
    fact(increment(3, 4)),
    rule(push(X, Y, Results), eq(Results, term(".", X, Y))),
)

assert solve_all(program(), X, onceo(eq(X, "first"))) == [atom("first")]
assert solve_all(program(), X, betweeno(1, 3, X)) == [num(1), num(2), num(3)]
assert solve_all(program(), X, succo(2, X)) == [num(3)]
assert solve_all(program(), X, conj(eq(X, 3), integero(X))) == [num(3)]
assert solve_all(
    program(),
    X,
    conj(disj(eq(X, "first"), eq(X, "second")), cuto()),
) == [atom("first")]
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
    compound_name_argumentso(X, "box", logic_list(["tea", "cake"])),
) == [term("box", "tea", "cake")]
assert solve_all(
    program(),
    (Name, Arity),
    compound_name_arityo(term("box", "tea"), Name, Arity),
) == [(atom("box"), num(1))]
assert solve_all(
    program(),
    X,
    univo(X, logic_list(["box", "tea", "cake"])),
) == [term("box", "tea", "cake")]
assert solve_all(program(), X, same_termo(X, X)) == [X]
assert solve_all(
    program(),
    (X, Score),
    conj(eq(X, term("pair", Y, Y)), numbervarso(X, 0, Score)),
) == [(term("pair", term("$VAR", 0), term("$VAR", 0)), num(1))]
assert solve_all(program(), Score, term_hash_boundedo(term("box", "tea"), 2, 1000, Score))
assert solve_all(program(), X, variant_termo(term("box", X), term("box", Y))) == [X]
assert solve_all(program(), X, subsumes_termo(term("box", X), term("box", "tea"))) == [X]
assert solve_all(program(), X, atom_charso(X, logic_list(["t", "e", "a"]))) == [atom("tea")]
assert solve_all(program(), X, atom_codeso("tea", X)) == [logic_list([116, 101, 97])]
assert solve_all(program(), X, number_charso(X, logic_list(["4", "2"]))) == [num(42)]
assert solve_all(program(), X, char_codeo(X, 90)) == [atom("Z")]
assert solve_all(program(), X, atom_concato("tea", "cup", X)) == [atom("teacup")]
assert solve_all(
    program(),
    X,
    atomic_list_concato_with_separator(logic_list(["tea", 2, "go"]), "-", X),
) == [atom("tea-2-go")]
assert solve_all(program(), X, number_stringo(X, string("3.5"))) == [num(3.5)]
assert solve_all(program(), X, atom_lengtho("teacup", X)) == [num(6)]
assert solve_all(program(), X, sub_atomo("teacup", 3, 3, 0, X)) == [atom("cup")]
assert solve_all(program(), X, sub_stringo(string("logic"), 2, 2, 1, X)) == [
    string("gi"),
]
assert solve_all(family, Body, clauseo(child("bart", "homer"), Body)) == [
    term("parent", "homer", "bart"),
]
assert solve_all(
    family,
    Body,
    conj(clauseo(child("bart", "homer"), Body), calltermo(Body)),
) == [term("parent", "homer", "bart")]
assert solve_all(program(), FlagValue, current_prolog_flago("unknown", FlagValue)) == [
    atom("fail"),
]
assert solve_all(
    program(),
    FlagValue,
    conj(
        set_prolog_flago("unknown", "error"),
        current_prolog_flago("unknown", FlagValue),
    ),
) == [atom("error")]
assert solve_all(
    higher_order,
    Results,
    maplisto("increment", logic_list([1, 2, 3]), Results),
) == [logic_list([2, 3, 4])]
assert solve_all(
    higher_order,
    (Name, Arity),
    partitiono("small", logic_list([1, 2, 3]), Name, Arity),
) == [(logic_list([1, 2]), logic_list([3]))]
assert solve_all(
    higher_order,
    Results,
    foldlo("push", logic_list(["a", "b", "c"]), logic_list([]), Results),
) == [logic_list(["c", "b", "a"])]
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
assert solve_all(
    program(),
    X,
    conj(fd_ino(X, range(1, 6)), fd_leqo(X, 3), fd_neqo(X, 2), labelingo([X])),
) == [num(1), num(3)]
assert solve_all(
    program(),
    (X, Y),
    conj(
        fd_ino(X, range(1, 5)),
        fd_ino(Y, range(1, 5)),
        fd_addo(X, Y, 5),
        all_differento([X, Y]),
        labelingo([X, Y]),
    ),
) == [(num(1), num(4)), (num(2), num(3)), (num(3), num(2)), (num(4), num(1))]
assert solve_all(
    program(),
    (X, Y),
    conj(
        fd_ino(X, range(1, 5)),
        fd_ino(Y, range(1, 5)),
        fd_sumo([X, Y, 1], 6),
        fd_lto(X, Y),
        labelingo([X, Y]),
    ),
) == [(num(1), num(4)), (num(2), num(3))]
```

Arithmetic is evaluative, not a constraint system yet. `iso(Y, add(X, 1))`
fails while `X` is unbound, and succeeds after a goal such as `eq(X, 4)` has
instantiated it.

Finite-domain constraints are branch-local and label explicitly. `fd_ino`
stores a finite integer domain; comparison, arithmetic, and all-different
predicates narrow domains as soon as enough information exists; and
`labelingo([X, Y])` enumerates concrete assignments in ascending order.
Arithmetic constraints currently cover addition, subtraction, and
multiplication, plus `fd_sumo` for n-ary sums and `fd_scalar_producto` for
weighted sums. Relation-aware variants support equality, disequality, and
ordering comparisons for sums and scalar products. `fd_elemento` adds a
1-based list-indexing global constraint. `all_differento` includes duplicate
checks and singleton pruning, while deeper Hall-set global-constraint pruning
remains future work.
`labelingo` chooses the smallest current finite domain first and uses the
caller-provided variable order as a stable tie-breaker.

## Finite-Domain Examples

The CLP(FD) layer is already useful enough to model recognizable logic
problems directly from Python. This map-coloring example solves a 3-coloring
for Australian regions:

```python
WA, NT, SA, Q, NSW, V, T = (
    var(name) for name in ("WA", "NT", "SA", "Q", "NSW", "V", "T")
)
regions = (WA, NT, SA, Q, NSW, V, T)
borders = (
    (WA, NT),
    (WA, SA),
    (NT, SA),
    (NT, Q),
    (SA, Q),
    (SA, NSW),
    (SA, V),
    (Q, NSW),
    (NSW, V),
)

assert solve_n(
    program(),
    1,
    regions,
    conj(
        *(fd_ino(region, range(1, 4)) for region in regions),
        fd_ino(WA, [1]),
        *(fd_neqo(left, right) for left, right in borders),
        labelingo(regions),
    ),
) == [(num(1), num(2), num(3), num(1), num(2), num(1), num(1))]
```

Scheduling, budgeting, and puzzle-style problems use the same ingredients:
finite start times, arithmetic relations for durations, `fd_sumo(...)` for
resource totals, ordering constraints for dependencies, and `labelingo(...)` to
enumerate concrete assignments.

Collections are observations over a nested proof search. `findallo` succeeds
with an empty list when the inner goal fails, while `bagofo` and `setofo` fail
for empty collections.

Advanced control is intentionally honest about the solver. `cuto()` is real
solver-level cut, not `onceo` in disguise: it prunes choicepoints made before it
in the current search frame while allowing choices created after it to continue
backtracking. `iftheno` and `ifthenelseo` commit to the first condition proof
while allowing the chosen branch to keep backtracking. `forallo` checks every
generated proof without leaking generator bindings to the outer query.

Term metaprogramming treats terms as ordinary data. `univo` decomposes
`box(tea, cake)` into `[box, tea, cake]` and can construct the term back from
that list. `functoro` now constructs atoms and compounds when supplied a name
and arity, while `compound_name_argumentso` and `compound_name_arityo` provide
compound-only name/arguments and name/arity reflection. `acyclic_termo` and
`cyclic_termo` expose standard finite-vs-rational-tree checks; today all
ordinary engine terms are acyclic because the core term model is immutable.
`unify_with_occurs_checko` exposes finite unification explicitly, while
`unifiableo` reports a unifier list without binding the source terms.
`copytermo` refreshes variables in a copied term, while `term_variableso`
extracts the unique variables still present after reification, `term_hasho`
gives variant-aware structural hashes for indexing and memoization, and
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
