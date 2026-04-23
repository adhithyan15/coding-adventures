# LP11 — Logic Builtins And Control

## Overview

The Python logic stack can now solve real relational problems:

- `logic-core` provides terms, substitutions, unification, and reification.
- `logic-engine` provides relations, facts, rules, recursion, and backtracking.
- `logic-stdlib` provides relational list helpers.
- `logic-instructions`, `logic-bytecode`, and their VMs provide executable data
  formats for the same model.

The next step toward Prolog-level functionality is not syntax. It is the
runtime vocabulary that real Prolog programs expect: practical control
predicates and type/term builtins.

LP11 introduces that layer.

## Design Goal

Add a Python `logic-builtins` package with a first set of Prolog-inspired
library predicates:

- `onceo(goal)`
- `noto(goal)` / negation as failure
- `callo(goal)`
- `groundo(term)`
- `varo(term)`
- `nonvaro(term)`
- `atomo(term)`
- `numbero(term)`
- `compoundo(term)`
- `functoro(term, name, arity)`
- `argo(index, term, value)`

These are library predicates, not Prolog syntax. They should compose with the
existing Python API and execute through the current solver.

## Why A Native Goal Hook Is Needed

Most current goals can be expressed with unification, conjunction, disjunction,
fresh variables, and relation calls. Some Prolog builtins cannot.

Examples:

- `var(X)` depends on whether `X` is currently unbound in the active
  substitution.
- `ground(T)` depends on whether the reified term contains any remaining logic
  variables.
- `once(G)` needs to run `G` and keep at most the first solution.
- negation as failure needs to ask whether `G` has no solutions from the current
  state.

That means `logic-engine` needs a small extension point for state-aware native
goals.

## Engine Extension

Extend `logic-engine` with:

```python
NativeGoalExpr
native_goal
solve_from
```

### `NativeGoalExpr`

`NativeGoalExpr` stores:

- a callable runner
- coerced term arguments

The runner receives:

- the active `Program`
- the current `State`
- the coerced term arguments

and yields successor states.

Conceptually:

```python
NativeGoalRunner = Callable[
    [Program, State, tuple[Term, ...]],
    Iterator[State],
]
```

### `native_goal(...)`

`native_goal(runner, *args)` should build a `NativeGoalExpr` with arguments
coerced through the same term coercion rules as `eq`, `neq`, and relation calls.

### `solve_from(...)`

`solve_from(program, goal, state)` should expose the existing internal
solve-from-state behavior so native goals can run nested goals from the current
state.

This is intentionally small. It does not expose the whole solver internals or a
mutable database.

## Package

Add a new Python package:

```text
code/packages/python/logic-builtins
```

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    ↓
LP01 Logic Engine
    ↓
LP11 Logic Builtins And Control   ← this milestone
```

The package should depend on `logic-engine` only.

## Public API

The package should export:

```python
callo
onceo
noto
groundo
varo
nonvaro
atomo
numbero
stringo
compoundo
functoro
argo
```

The `o` suffix keeps the miniKanren-style convention used by `logic-stdlib`:
these functions return goals.

## Semantics

### `callo(goal)`

Runs a goal expression supplied as data.

This is mostly a small adapter for composability. In Python, callers already
have goal objects, so `callo(goal)` should behave like `goal`.

### `onceo(goal)`

Runs `goal` and yields only the first solution, if any.

This is a committed-choice helper. It does not mutate global state and it does
not implement Prolog cut.

### `noto(goal)`

Implements negation as failure:

- succeeds once if `goal` has no solutions from the current state
- fails if `goal` has at least one solution from the current state

This is operational negation, not classical logical negation.

### `groundo(term)`

Succeeds when the current reified value of `term` contains no unbound logic
variables.

### `varo(term)`

Succeeds when the current reified value of `term` is still an unbound logic
variable.

### `nonvaro(term)`

Succeeds when the current reified value of `term` is not an unbound logic
variable.

### `atomo(term)`

Succeeds when the current reified value is an `Atom`.

### `numbero(term)`

Succeeds when the current reified value is a `Number`.

### `stringo(term)`

Succeeds when the current reified value is a `String`.

### `compoundo(term)`

Succeeds when the current reified value is a `Compound`.

### `functoro(term, name, arity)`

For this first slice, support inspection mode:

- `term` must currently be a compound
- `name` unifies with the compound functor as an atom
- `arity` unifies with the compound arity as a number

Construction mode can come later.

### `argo(index, term, value)`

For this first slice, support inspection mode:

- `term` must currently be a compound
- `index` must currently be a positive integer number
- `value` unifies with the 1-based argument at `index`

Construction mode can come later.

## Error Model

The builtins should prefer logical failure over Python exceptions for ordinary
predicate failure.

Python exceptions are appropriate for API misuse, such as:

- passing a non-goal to `callo`, `onceo`, or `noto`
- passing host booleans where logic terms are expected

## Test Strategy

Required tests:

- `onceo` keeps only the first solution
- `noto` succeeds when a goal fails and fails when a goal succeeds
- `callo` runs a supplied goal
- `groundo` distinguishes ground terms from terms with unbound variables
- `varo` and `nonvaro` observe the current binding state
- atom, number, string, and compound type checks work
- `functoro` extracts compound functor and arity
- `argo` extracts 1-based compound arguments
- builtins compose with ordinary relations and list relations

## Future Extensions

Later functionality-level Prolog milestones should add:

- arithmetic evaluation and comparisons
- `findall`, `bagof`, and `setof`
- cut and if-then-else
- dynamic predicate database operations
- better indexing and tabling
- CLP(FD)-style finite-domain constraints

## Summary

LP11 is the first practical Prolog builtins layer for the library-first design.

It keeps syntax out of scope, but gives Python users state-aware predicates and
control helpers that real Prolog programs rely on.
