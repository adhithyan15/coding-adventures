# LP03 — Relational Standard Library

## Overview

`logic-core` gives us terms, unification, and search.
`logic-engine` gives us relations, clauses, and recursive solving.

That is enough to *express* many logic programs, but it is still missing the
small reusable vocabulary that makes relational programming pleasant in a
host-language library.

Right now, if a user wants a `member` relation or list concatenation, they have
to hand-write the rules every time:

```python
member = relation("member", 2)

members = program(
    rule(member(x, term(".", x, tail)), succeed()),
    rule(member(x, term(".", head, tail)), member(x, tail)),
)
```

That is educational once. It is noise after that.

This milestone adds a tiny standard library package of reusable relational
helpers built directly on top of `logic-engine`.

## Design Goal

Add the smallest useful relational vocabulary that lets users solve list-shaped
logic problems directly in Python without first building their own helper
relations.

The first slice should include:

- `emptyo`
- `conso`
- `heado`
- `tailo`
- `membero`
- `appendo`

Because Python helper functions are evaluated eagerly, this milestone also
depends on one small engine capability:

- `defer(builder, *args)` for solve-time recursive goal expansion

The naming follows the common miniKanren-style `o` suffix to signal that these
are *relations*, not ordinary Python functions.

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    ↓
LP01 Logic Engine
    ↓
LP02 Disequality Constraints
    ↓
LP03 Relational Standard Library   ← this milestone
    - list relations
    - reusable host-language vocabulary
```

## Why A Separate Package

These helpers should live in a new package rather than in `logic-engine`
itself.

That keeps the stack clean:

- `logic-engine` remains the execution substrate
- `logic-stdlib` becomes reusable vocabulary on top of that substrate

This is a good fit for the larger project vision:

- the engine stays small and language-neutral
- the standard library can grow incrementally
- a future Prolog implementation can still compile down to the same engine

## First API

The first Python package should expose:

```python
emptyo(value: object) -> GoalExpr
conso(head: object, tail: object, pair: object) -> GoalExpr
heado(items: object, head: object) -> GoalExpr
tailo(items: object, tail: object) -> GoalExpr
membero(member: object, items: object) -> GoalExpr
appendo(left: object, right: object, combined: object) -> GoalExpr
```

These all return `logic-engine` goal expressions and should compose with:

- `conj(...)`
- `disj(...)`
- `fresh(...)`
- `defer(...)`
- `neq(...)`
- relation calls from user-defined programs

## Semantics

### `emptyo`

Succeed exactly when the value is the empty list.

```python
emptyo(X)
```

means:

```python
eq(X, [])
```

### `conso`

Relate a list to its head and tail.

```python
conso(H, T, L)
```

means:

```text
L = .(H, T)
```

This is the fundamental list-construction relation.

### `heado`

Expose the first element of a non-empty list.

```python
heado([tea, cake], X)
⇒ X = tea
```

### `tailo`

Expose the tail of a non-empty list.

```python
tailo([tea, cake], X)
⇒ X = [cake]
```

### `membero`

Relate an element to a list when the element appears anywhere in that list.

Examples:

```python
membero(X, [tea, cake])
⇒ X = tea ; X = cake

membero(cake, [tea, cake])
⇒ success
```

Operationally, the first version should encode the standard recursive rule:

```text
membero(X, [X | _]).
membero(X, [_ | Tail]) :- membero(X, Tail).
```

### `appendo`

Relate two lists to their concatenation.

Examples:

```python
appendo([tea], [cake], X)
⇒ X = [tea, cake]

appendo(X, Y, [tea, cake])
⇒ X = [], Y = [tea, cake]
⇒ X = [tea], Y = [cake]
⇒ X = [tea, cake], Y = []
```

Operationally, the first version should encode:

```text
appendo([], Right, Right).
appendo([Head | LeftTail], Right, [Head | OutTail]) :-
    appendo(LeftTail, Right, OutTail).
```

## Representation

The package should reuse the canonical list representation already established
in `logic-core`:

- `[]` is the atom `[]`
- non-empty lists are nested `.(Head, Tail)` terms

The standard library must not invent a second list encoding.

## Package

Create a new Python package:

```text
code/packages/python/logic-stdlib
```

Suggested publishable package metadata:

- distribution: `coding-adventures-logic-stdlib`
- module: `logic_stdlib`

Dependencies:

- `coding-adventures-logic-engine`

## Usage Example

```python
from logic_engine import logic_list, program, solve_n, var
from logic_stdlib import appendo

prefix = var("Prefix")
suffix = var("Suffix")

answers = solve_n(
    program(),
    3,
    (prefix, suffix),
    appendo(prefix, suffix, logic_list(["tea", "cake"])),
)
```

The answers should be:

```text
([], [tea, cake])
([tea], [cake])
([tea, cake], [])
```

## Search Notes

The standard library inherits `logic-engine`'s current execution strategy:

- depth-first
- left-biased
- no tabling

That means some highly open-ended queries may diverge. This is acceptable for
the first slice as long as:

- the relations are correct
- the docs use bounded examples where appropriate
- tests focus on finite or deliberately truncated queries

The package should use `defer(...)` internally for recursive relations such as
`membero` and `appendo`, so users get recursion without manually building
auxiliary clause databases.

## Test Strategy

Required tests:

- `emptyo` recognizes the empty list
- `conso` can construct and deconstruct lists
- `heado` extracts the first list element
- `tailo` extracts the remaining list tail
- `membero` enumerates members of a concrete list
- `appendo` concatenates concrete lists
- `appendo` can split a concrete list into prefixes and suffixes

## Future Extensions

Later milestones may add:

- `listo`
- `lengtho`
- `reverseo`
- arithmetic relations
- finite-domain helpers

This milestone should stay intentionally small and crisp.

## Why This Milestone Matters

LP03 is where the Python library starts to feel like a genuine relational
toolkit instead of just a raw execution engine.

Users will be able to write:

- list decomposition problems
- prefix/suffix search problems
- membership constraints
- small symbolic puzzles

without first re-deriving the same recursive list relations every time.
