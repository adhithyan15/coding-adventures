# LP04 — Relational Combinators For Choice And Permutation

## Overview

`logic-stdlib` now gives us:

- `emptyo`
- `conso`
- `heado`
- `tailo`
- `membero`
- `appendo`

That is enough for basic list decomposition and concatenation problems.

It is still awkward to express a very common class of logic problems:

- pick one value from a set of candidates
- continue solving with the remaining candidates
- enumerate permutations of a list
- model seating orders, assignments, and ordering puzzles

This milestone adds the next small but powerful vocabulary for those tasks.

## Design Goal

Add the smallest combinatorial relations that unlock a large family of logic
problems without introducing a new evaluator or a constraint system.

The first slice should include:

- `selecto`
- `permuteo`

These should live in `logic-stdlib`, continue using `logic-engine` goal
expressions, and rely on the existing `defer(...)` mechanism for recursive
host-language expansion.

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
LP03 Relational Standard Library
    ↓
LP04 Relational Combinators   ← this milestone
    - choice from a list
    - permutation search
```

## Why These Two Relations

`selecto` is the relational form of:

```text
pick one item and return the remaining list
```

`permuteo` is the relational form of:

```text
reorder a list in every possible way
```

Together they make it much easier to express:

- “assign each person a unique seat”
- “try each ordering of these values”
- “choose one unused option and recurse”
- “search all possible arrangements, then filter with constraints”

## API

Extend `logic-stdlib` with:

```python
selecto(member: object, items: object, remainder: object) -> GoalExpr
permuteo(items: object, permutation: object) -> GoalExpr
```

These should compose naturally with:

- `membero(...)`
- `appendo(...)`
- `neq(...)`
- `all_different(...)`
- `conj(...)`
- `disj(...)`

## Semantics

### `selecto`

`selecto(X, Items, Rest)` means:

> `X` is one element of `Items`, and `Rest` is `Items` with that one occurrence
> removed.

Examples:

```python
selecto(X, [tea, cake, jam], Rest)
⇒ X = tea, Rest = [cake, jam]
⇒ X = cake, Rest = [tea, jam]
⇒ X = jam, Rest = [tea, cake]
```

Operationally, the first version should encode:

```text
selecto(X, [X | Tail], Tail).
selecto(X, [Head | Tail], [Head | RestTail]) :-
    selecto(X, Tail, RestTail).
```

This is the relational building block for “use one choice and keep the
remaining choices for later.”

### `permuteo`

`permuteo(Items, Permutation)` means:

> `Permutation` is one ordering of the elements of `Items`.

Examples:

```python
permuteo([red, green, blue], X)
⇒ X = [red, green, blue]
⇒ X = [red, blue, green]
⇒ X = [green, red, blue]
⇒ ...
```

Operationally, the first version should encode:

```text
permuteo([], []).
permuteo(Items, [Head | Tail]) :-
    selecto(Head, Items, Remaining),
    permuteo(Remaining, Tail).
```

## Why This Matters For Logic Problems

Many hand-solved logic puzzles are fundamentally “generate permutations, then
constrain them.”

With `permuteo(...)` and existing disequality support, users can write much more
natural host-language logic programs for:

- seating arrangements
- small scheduling problems
- ordering puzzles
- cryptarithmetic-style candidate assignment scaffolds

This is still library-first. We are improving the reusable substrate that the
future Prolog frontend will compile into.

## Package Impact

This milestone updates the existing Python package:

```text
code/packages/python/logic-stdlib
```

No new package is needed. These combinators are a natural extension of the
existing relational standard library.

## Usage Example

```python
from logic_engine import logic_list, program, solve_n, var
from logic_stdlib import permuteo

Order = var("Order")

answers = solve_n(
    program(),
    3,
    Order,
    permuteo(logic_list(["tea", "cake", "jam"]), Order),
)
```

The answers should be the first three permutations in search order.

## Search Notes

The current engine is still:

- depth-first
- left-biased
- not tabled

That means open-ended permutation queries can get expensive quickly. This is
acceptable for the first slice as long as:

- examples stay small
- tests use concrete finite inputs
- docs encourage bounded search with `solve_n(...)` where helpful

## Test Strategy

Required tests:

- `selecto` can remove a known element from a concrete list
- `selecto` can enumerate all element/remainder pairs of a concrete list
- `permuteo` enumerates every permutation of a small list
- `permuteo` works as a relational helper inside ordinary `solve_all(...)`

## Future Extensions

Later milestones may add:

- `lengtho`
- `subsequenceo`
- finite-domain labeling helpers
- puzzle-specific helper packages

## Why This Milestone Matters

LP03 made the library feel like a usable relational list toolkit.

LP04 makes it feel much closer to a real logic-problem workbench by adding the
combinatorial relations that many puzzle and search problems naturally need.
