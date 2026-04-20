# LP13 - Logic Collection Builtins

## Overview

LP11 added Prolog-style control and term inspection. LP12 added evaluative
arithmetic. The next practical runtime feature is solution collection.

Real Prolog programs often need to move between two views of a relation:

- as a stream of individual answers produced by backtracking
- as one concrete list of answers that can be inspected, counted, filtered, or
  passed to later goals

LP13 adds the first collection predicates for that bridge.

## Design Goal

Extend the Python `logic-builtins` package with library-level collection goals:

- `findallo(template, goal, results)`
- `bagofo(template, goal, results)`
- `setofo(template, goal, results)`

These are goal constructors, not syntax. They should run through the existing
solver and return ordinary `logic-engine` goals.

## Package

Update:

```text
code/packages/python/logic-builtins
```

The package version should move from `0.2.0` to `0.3.0`.

## Public API

Add:

```python
findallo(template, goal, results)
bagofo(template, goal, results)
setofo(template, goal, results)
```

The `o` suffix keeps the package convention that public predicates return
logic goals.

## Semantics

### `findallo(template, goal, results)`

Run `goal` from the current solver state. For every solution, reify
`template` under that solution's substitution, preserving proof order. Unify
`results` with a logic list containing those reified template values.

`findallo` succeeds once even when the goal has no solutions:

```python
findallo(X, fail(), Results)  # Results = []
```

Bindings produced while proving the inner goal should not leak to the outer
state except through the collected result list.

### `bagofo(template, goal, results)`

`bagofo` is the first bag-style collector. It preserves duplicates and proof
order like `findallo`, but it fails when the inner goal has no solutions.

This first slice does not yet implement Prolog's full free-variable grouping or
existential quantification rules. Later language-level work can add that richer
mode once the runtime has a first-class representation for quantified goal
variables.

### `setofo(template, goal, results)`

`setofo` collects all template values, removes duplicates, sorts them by a
stable term ordering, and unifies `results` with the resulting logic list. Like
`bagofo`, it fails when the inner goal has no solutions.

The LP13 ordering is intentionally simple and deterministic:

1. variables
2. numbers
3. atoms
4. strings
5. compound terms, ordered by functor and recursively by arguments

This is close enough for a practical first library layer. A later Prolog
compatibility milestone can refine it toward the exact standard order of terms.

## State Discipline

Collection predicates are state-aware native goals. Each collector should:

1. validate the supplied `goal` as a goal expression
2. solve it from the active outer `State`
3. reify the `template` under each inner solution state
4. build a canonical Prolog-style list with `logic_list(...)`
5. unify the requested `results` term against that list from the original outer
   state

That final step is important. Inner goal bindings are observations used to
build the collection. They should not escape as ordinary outer bindings.

## Error Model

As with LP11 control predicates, passing a non-goal to any collection predicate
is host-language API misuse and should raise `TypeError`.

Ordinary logical cases should stay logical:

- empty `findallo` succeeds with `[]`
- empty `bagofo` fails
- empty `setofo` fails
- result unification failure causes the collector to fail

## Test Strategy

Required tests:

- `findallo` collects multiple answers in proof order
- `findallo` preserves duplicates
- `findallo` succeeds with an empty list when the inner goal fails
- `findallo` does not leak inner goal bindings outside the result list
- `bagofo` preserves duplicates and fails on no solutions
- `setofo` removes duplicates and produces deterministic ordering
- collectors compose with arithmetic predicates
- collectors compose with relation search
- collectors reject non-goals

## Future Extensions

Later collection-related milestones can add:

- full Prolog `bagof/3` free-variable grouping
- `^` existential quantification for `bagof/3` and `setof/3`
- standard term ordering compatibility
- count-oriented helpers such as `counto`
- bytecode/VM instructions for collection goals

## Summary

LP13 turns backtracking answers into first-class logic lists. This is a major
step toward practical Prolog-level library programming because callers can now
ask both "what are the solutions?" and "give me the solutions as data" without
leaving the relational API.
