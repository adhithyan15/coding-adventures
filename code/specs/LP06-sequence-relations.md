# LP06 — Sequence Relations

## Overview

`logic-stdlib` already gives us:

- `emptyo`
- `conso`
- `heado`
- `tailo`
- `listo`
- `membero`
- `appendo`
- `selecto`
- `permuteo`
- `reverseo`

That means the Python host-language library can already:

- deconstruct lists
- validate proper lists
- concatenate lists
- enumerate members
- choose one element from a list
- generate permutations
- relate a list to its reversal

There is still a useful gap around ordered sequence reasoning:

- there is no reusable relation for the final element of a list
- there is no reusable relation for order-preserving subsequences

Those relations are small, but they appear constantly in textbook logic
programs, interview-style list puzzles, and parser-style sequence reasoning.

## Design Goal

Add the next pair of sequence-oriented relations:

- `lasto`
- `subsequenceo`

These should stay inside `logic-stdlib`, compile to ordinary `logic-engine`
goal expressions, and use `defer(...)` for recursive host-language expansion.

## Why Not `lengtho` Yet

`lengtho` is a natural future relation, but a satisfying version wants a clear
story for arithmetic and bidirectional numeric reasoning.

We do not have that yet. Rather than introducing a half-relational `lengtho`
that only works comfortably in one direction, this milestone focuses on two
sequence relations that are already a good fit for the current engine.

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
LP05 Structural List Relations
    ↓
LP06 Sequence Relations   ← this milestone
    - final-element reasoning
    - ordered subsequence reasoning
```

## API

Extend `logic-stdlib` with:

```python
lasto(items: object, last: object) -> GoalExpr
subsequenceo(items: object, subsequence: object) -> GoalExpr
```

These should compose naturally with:

- `membero(...)`
- `appendo(...)`
- `selecto(...)`
- `permuteo(...)`
- `reverseo(...)`
- `neq(...)`
- `conj(...)`
- `disj(...)`

## Semantics

### `lasto`

`lasto(Items, Last)` means:

> `Last` is the final element of the non-empty proper list `Items`.

Examples:

```python
lasto([tea, cake, jam], X)
⇒ X = jam

lasto([tea], X)
⇒ X = tea
```

Operationally, the first version should encode:

```text
lasto([Last], Last).
lasto([_Head | Tail], Last) :-
    lasto(Tail, Last).
```

This relation is especially useful because users often want to talk about
“whatever comes last” without manually writing a recursive list walker in every
example.

### `subsequenceo`

`subsequenceo(Items, Subsequence)` means:

> `Subsequence` is formed by deleting zero or more elements from `Items`
> without changing the relative order of the remaining elements.

This is not a contiguous-slice relation. It is an order-preserving deletion
relation.

Examples:

```python
subsequenceo([tea, cake, jam], X)
⇒ X = [tea, cake, jam]
⇒ X = [tea, cake]
⇒ X = [tea, jam]
⇒ X = [cake]
⇒ X = []
```

Operationally, the first version should encode:

```text
subsequenceo([], []).
subsequenceo([Head | Tail], [Head | SubTail]) :-
    subsequenceo(Tail, SubTail).
subsequenceo([_Head | Tail], Subsequence) :-
    subsequenceo(Tail, Subsequence).
```

The keep-first branch should come before the drop branch so the search order
prefers larger, less-deleted subsequences before smaller ones.

## Why This Matters

These two relations help the host-language library feel more like an actual
logic-programming toolkit instead of a bag of isolated examples.

They make it easier to express:

- “the answer must appear at the end”
- “this pattern appears in order, though not necessarily contiguously”
- “generate candidates by deleting some steps while preserving order”
- “filter ordered arrangements without collapsing back into manual Python”

## Package Impact

This milestone updates the existing Python package:

```text
code/packages/python/logic-stdlib
```

No new package is needed.

## Usage Example

```python
from logic_engine import atom, logic_list, program, solve_all, var
from logic_stdlib import lasto, subsequenceo

X = var("X")

assert solve_all(
    program(),
    X,
    lasto(logic_list(["tea", "cake", "jam"]), X),
) == [atom("jam")]

assert solve_all(
    program(),
    X,
    subsequenceo(logic_list(["tea", "cake"]), X),
) == [
    logic_list(["tea", "cake"]),
    logic_list(["tea"]),
    logic_list(["cake"]),
    logic_list([]),
]
```

## Search Notes

The current engine is still:

- depth-first
- left-biased
- not tabled

So open-ended inverse sequence queries can still diverge. The first slice
should keep tests and examples finite by:

- using concrete source lists
- validating known outputs or enumerating from bounded inputs
- avoiding large open-ended reverse or subsequence synthesis

## Test Strategy

Required tests:

- `lasto` extracts the final element of a concrete list
- `lasto` works for a single-element list
- `subsequenceo` enumerates all subsequences of a small concrete list
- `subsequenceo` can validate a known order-preserving subsequence
- `subsequenceo` rejects an out-of-order candidate

## Future Extensions

Later milestones may add:

- `lengtho`
- `prefixo`
- `suffixo`
- finite-domain arithmetic relations
- puzzle-specific helper packages

## Why This Milestone Matters

LP05 made the library stronger at structural list reasoning.

LP06 makes it stronger at talking about ordered sequence structure, which is a
very common shape in logic problems and later Prolog examples.
