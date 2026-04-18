# LP05 — Structural List Relations

## Overview

`logic-stdlib` already gives us:

- `emptyo`
- `conso`
- `heado`
- `tailo`
- `membero`
- `appendo`
- `selecto`
- `permuteo`

That covers:

- list construction and deconstruction
- membership search
- concatenation
- choice from a list
- permutation search

There is still an awkward gap in the host-language library:

- there is no direct way to say that a value is a proper list
- there is no standard relation for list reversal

Those ideas are small, but they show up constantly in logic programming
examples and puzzle-style code. This milestone fills that gap.

## Design Goal

Add the next pair of foundational structural relations for lists:

- `listo`
- `reverseo`

These should remain ordinary `logic-engine` goal expressions and should keep
using `defer(...)` for recursive host-language expansion.

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
LP04 Relational Combinators
    ↓
LP05 Structural List Relations   ← this milestone
    - proper-list recognition
    - list reversal
```

## Why These Two Relations

`listo` is the relational form of:

```text
this value is a proper list
```

`reverseo` is the relational form of:

```text
these two lists are reverses of one another
```

Together they make it easier to:

- validate recursive list-shaped data
- write structure-preserving list programs
- express reverse-order constraints relationally
- teach the difference between proper lists and arbitrary dotted pairs

## API

Extend `logic-stdlib` with:

```python
listo(value: object) -> GoalExpr
reverseo(items: object, reversed_items: object) -> GoalExpr
```

These should compose naturally with:

- `conso(...)`
- `appendo(...)`
- `membero(...)`
- `selecto(...)`
- `permuteo(...)`
- `neq(...)`
- `conj(...)`
- `disj(...)`

## Semantics

### `listo`

`listo(Value)` means:

> `Value` is the canonical empty list, or a cons cell whose tail is itself a
> proper list.

Examples:

```python
listo([])
⇒ success

listo([tea, cake])
⇒ success

listo([tea | cake])
⇒ failure
```

Operationally, the first version should encode:

```text
listo([]).
listo([_Head | Tail]) :-
    listo(Tail).
```

This relation matters because the host-language library currently represents
lists using ordinary terms. That is powerful, but it also means users can build
improper dotted pairs. `listo(...)` is the reusable relation that marks the
difference.

### `reverseo`

`reverseo(Items, ReversedItems)` means:

> `ReversedItems` contains exactly the elements of `Items` in the opposite
> order.

Examples:

```python
reverseo([tea, cake, jam], X)
⇒ X = [jam, cake, tea]

reverseo(X, [jam, cake, tea])
⇒ X = [tea, cake, jam]
```

Operationally, the first version should encode:

```text
reverseo([], []).
reverseo([Head | Tail], Reversed) :-
    reverseo(Tail, ReversedTail),
    appendo(ReversedTail, [Head], Reversed).
```

The first implementation may use `appendo(...)` plus a one-element list
constructed through `logic_list([Head])`.

## Why This Matters For The Library-First Vision

The host-language library should be useful before the Prolog frontend exists.

This milestone improves that story in two ways:

- `listo(...)` gives users a standard structural invariant they can assert in
  ordinary Python logic programs
- `reverseo(...)` adds a classic recursive relation that many textbooks and
  tutorials use to explain relational evaluation

That makes the library more educational and more practical at the same time.

## Package Impact

This milestone updates the existing Python package:

```text
code/packages/python/logic-stdlib
```

No new package is needed. These relations are a natural extension of the
current relational standard library.

## Usage Example

```python
from logic_engine import logic_list, program, solve_all, var
from logic_stdlib import listo, reverseo

Items = var("Items")

assert solve_all(
    program(),
    Items,
    reverseo(logic_list(["tea", "cake", "jam"]), Items),
) == [logic_list(["jam", "cake", "tea"])]
```

## Search Notes

The current engine is still:

- depth-first
- left-biased
- not tabled

So some open-ended structural queries can diverge or grow quickly. The first
slice should keep examples and tests bounded by:

- using concrete finite lists in tests
- preferring validation and one-directional construction examples
- avoiding large open-ended reverse enumeration

## Test Strategy

Required tests:

- `listo` succeeds for the empty list
- `listo` succeeds for a concrete proper list
- `listo` fails for a concrete improper dotted pair
- `reverseo` reverses a concrete list
- `reverseo` can validate a known reverse ordering inside a larger goal

## Future Extensions

Later milestones may add:

- `lengtho`
- `prefixo`
- `suffixo`
- finite-domain arithmetic relations
- puzzle-specific helper packages

## Why This Milestone Matters

LP04 made the library good at combinatorial list search.

LP05 makes it better at structural list reasoning, which is a big part of how
logic programming is taught and how many elegant list relations are built.
