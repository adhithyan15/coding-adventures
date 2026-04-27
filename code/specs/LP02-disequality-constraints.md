# LP02 — Disequality Constraints For Real Logic Problems

## Overview

`logic-core` and `logic-engine` can now express:

- terms
- unification
- facts and rules
- recursive relation solving

That is enough for relation expansion problems like family trees and graph
reachability.

It is **not** enough for many real logic problems, because those need to say:

> these two things must **not** become equal

Examples:

- two neighboring regions on a map must have different colors
- two people cannot occupy the same seat
- two houses in a logic puzzle cannot share the same attribute value
- a value must not equal a forbidden choice

This milestone adds that missing idea.

## Design Goal

Add the smallest correct constraint feature that materially increases what the
library can solve:

- delayed disequality constraints
- constraint-aware search states
- engine support for `neq(...)`
- a convenience helper for pairwise distinct values

This keeps the roadmap engine-first and puzzle-oriented without jumping yet to
finite-domain solving or full CLP.

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    - terms
    - unification
    - goals
    - state / search
    ↓
LP01 Logic Engine
    - relations
    - clauses
    - recursive resolution
    ↓
LP02 Disequality Constraints   ← this milestone
    - neq
    - constraint store
    - all_different helper
```

## What Problem This Solves

Consider a map-coloring fragment:

```python
color = relation("color", 1)

goal = conj(
    color(WA),
    color(NT),
    color(SA),
    neq(WA, NT),
    neq(WA, SA),
    neq(NT, SA),
)
```

Without disequality, the engine happily gives:

```text
WA = red, NT = red, SA = red
```

because nothing forbids those values from collapsing together.

With disequality constraints, that assignment is rejected and the solver keeps
searching for assignments where adjacent variables stay distinct.

## Core Semantics

### Immediate Failure

If two terms are already equal under the current substitution:

```python
neq(atom("red"), atom("red"))
```

then the goal fails immediately.

### Immediate Success

If two terms can already be proven different:

```python
neq(atom("red"), atom("blue"))
neq(term("f", "x"), term("g", "x"))
```

then the goal succeeds immediately and stores no new constraint.

### Delayed Constraint

If the terms are not equal now, but *could* become equal later depending on how
variables are bound, the solver must keep a pending constraint.

Examples:

```python
neq(X, atom("red"))
neq(X, Y)
neq(term("pair", X, atom("a")), term("pair", atom("red"), Y))
```

These should succeed for now, but add a constraint to the search state.

## Constraint Store

The search `State` should gain a constraint store.

The first version only needs one constraint kind:

- `Disequality(left, right)`

Conceptually:

```python
@dataclass(frozen=True, slots=True)
class State:
    substitution: Substitution
    constraints: tuple[Disequality, ...]
    next_var_id: int
```

Whenever the substitution changes, the stored disequalities must be rechecked.

Each stored constraint then lands in one of three categories:

1. **violated** — the two sides are equal now → fail the state
2. **satisfied** — the two sides can no longer unify → drop the constraint
3. **still pending** — equality is not forced yet, but still possible → keep it

## API Changes

### In `logic-core`

Add:

```python
Disequality
neq(left: object, right: object) -> Goal
```

`eq(...)` must become constraint-aware. A successful unification is only valid
if all stored disequalities remain consistent with the new substitution.

### In `logic-engine`

Add:

```python
NeqExpr
neq(left: object, right: object) -> GoalExpr
all_different(*terms: object) -> GoalExpr
```

`all_different` is a convenience combinator that lowers to pairwise `neq(...)`
constraints:

```text
all_different(A, B, C)
⇒ conj(neq(A, B), neq(A, C), neq(B, C))
```

## Constraint-Aware Search

The search model stays:

- depth-first
- left-biased
- generator-based

The new behavior is that states now carry delayed disequalities alongside the
substitution.

This means backtracking automatically restores both:

- the previous substitution
- the previous constraint store

That is exactly what we want from a persistent state model.

## Interaction With Reification

Reification should continue to resolve query terms as before.

The first version does **not** need to expose unresolved constraints in the
returned answer values. Returning the reified query terms is enough.

Advanced callers can still inspect raw `State` values if they want to see the
remaining pending constraints.

## Engine Example

```python
from logic_engine import (
    all_different,
    fact,
    conj,
    program,
    relation,
    solve_n,
    var,
)

color = relation("color", 1)

palette = program(
    fact(color("red")),
    fact(color("green")),
    fact(color("blue")),
)

WA = var("WA")
NT = var("NT")
SA = var("SA")

answers = solve_n(
    palette,
    3,
    (WA, NT, SA),
    conj(
        color(WA),
        color(NT),
        color(SA),
        all_different(WA, NT, SA),
    ),
)
```

The answers should include only colorings where the three variables are pairwise
different.

## What This Milestone Does NOT Include Yet

To keep scope focused, LP02 does not yet add:

- finite-domain propagation
- arithmetic constraints
- CLP(FD)
- reified constraints
- optimization / labeling strategies
- tabling
- negation-as-failure

This is just the first, smallest useful constraint feature.

## Packages Affected

This milestone should update the existing Python packages:

- `code/packages/python/logic-core`
- `code/packages/python/logic-engine`

No new package is required yet because disequality is fundamental enough to
belong directly in the core search model and the engine expression layer.

## Test Strategy

Required tests:

- `neq` fails on already-equal terms
- `neq` succeeds immediately on obviously different terms
- `neq` stores a delayed constraint for undecided terms
- later `eq` that violates a stored constraint fails
- later `eq` that satisfies a stored constraint drops it
- `logic-engine` can use `neq` inside solved goals
- `all_different` works on a small assignment / coloring example

## Why This Milestone Matters

LP01 proved that the repo can solve recursive relational programs.

LP02 makes those programs far more useful by adding the first real constraint
mechanism. That is the step that starts turning the Python library into a
genuine logic-problem toolkit rather than only a relation expander.
