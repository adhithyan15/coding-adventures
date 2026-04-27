# LP15 - Logic Term Metaprogramming Builtins

## Overview

LP11 through LP14 made the Python logic stack useful for real relational
programming: control predicates, type checks, arithmetic, collections, and
advanced committed-choice helpers now exist as library goals. The next
Prolog-level functionality gap is metaprogramming over terms.

Prolog programs often treat clauses and goals as data. Before the parser layer
exists, the Python library should still let callers inspect, construct, and
copy logic terms in a Prolog-shaped way.

LP15 extends `logic-builtins` with term metaprogramming predicates.

## Design Goal

Add a first library layer for working with terms as data:

- decompose and construct terms with `univo`, the library spelling of `=../2`
- extend `functoro` from inspection-only into construction mode
- copy terms while refreshing variables
- test strict term identity without unifying
- add small atomic/callable classification helpers

These predicates should compose with the existing `logic-engine` solver and
should remain ordinary goal constructors, not syntax.

## Package

Update:

```text
code/packages/python/logic-builtins
```

The package version should move from `0.4.0` to `0.5.0`.

## Public API

Add:

```python
univo(term, parts)
copytermo(source, copy)
same_termo(left, right)
atomico(term)
callableo(term)
```

Extend:

```python
functoro(term, name, arity)
```

## Semantics

### `univo(term, parts)`

`univo` is the library spelling of Prolog's `=../2`.

In decomposition mode, when `term` is instantiated:

- a compound term `box(tea, cake)` decomposes to `[box, tea, cake]`
- an atomic term such as `tea`, `3`, or `"tea"` decomposes to `[tea]`, `[3]`,
  or `["tea"]`

In construction mode, when `term` is unbound and `parts` is a proper list:

- a one-item list constructs that atomic item
- a multi-item list constructs a compound whose first item is an atom functor
  and whose remaining items are arguments
- an empty list fails
- a multi-item list whose first item is not an atom fails

LP15 does not support partial-list generation. `parts` must either be a proper
list after reification or be unified from an instantiated `term`.

### `functoro(term, name, arity)`

`functoro` should now support both inspection and construction.

Inspection mode:

- for a compound, unify `name` with its functor atom and `arity` with its
  argument count
- for an atomic term, unify `name` with the term itself and `arity` with `0`

Construction mode:

- if `term` is unbound, `name` is instantiated, and `arity` is a non-negative
  integer number, build a term
- `arity == 0` builds the atomic `name`
- `arity > 0` requires `name` to be an atom and builds a compound with fresh
  logic variables as arguments

This mirrors the useful Prolog behavior while staying finite and predictable.

### `copytermo(source, copy)`

`copytermo` duplicates a term and replaces every unbound variable in the source
with a fresh variable in the copy. Repeated references to the same source
variable must map to the same fresh copy variable.

Bindings that already exist in the active substitution should be respected
before copying. In other words, `copytermo(X, Copy)` after `X = box(Y)` copies
`box(Y)`, not the original unbound `X`.

### `same_termo(left, right)`

`same_termo` is strict identity/equality over reified terms. It succeeds when
the two reified terms are structurally equal, including variable identity. It
does not bind variables.

This is the library spelling of Prolog's `==/2`, not unification.

### `atomico(term)`

Succeeds when the current reified term is atomic:

- atom
- number
- string

Unbound variables and compound terms fail.

### `callableo(term)`

Succeeds when the current reified term can be called as a goal-like term:

- atom
- compound

Numbers, strings, and unbound variables fail in this first slice.

## Error Model

These predicates should use logical failure for ordinary mode failures:

- invalid `univo` parts fail
- construction-mode `functoro` with a non-integer or negative arity fails
- construction-mode `functoro` with a positive arity and non-atom functor fails

Host-language coercion errors from unsupported Python objects may still raise
`TypeError`, following the existing package convention.

## Test Strategy

Required tests:

- `univo` decomposes compounds in functor-first order
- `univo` decomposes atomic values to singleton lists
- `univo` constructs compounds from proper lists
- `univo` constructs atomic values from singleton lists
- `univo` fails for empty lists and invalid compound functors
- `functoro` inspects compound and atomic terms
- `functoro` constructs atoms for arity zero
- `functoro` constructs compounds with fresh argument variables
- `functoro` fails for invalid construction inputs
- `copytermo` copies ground terms unchanged
- `copytermo` refreshes variables while preserving aliasing inside the copy
- `same_termo` observes strict equality without binding variables
- `atomico` and `callableo` classify reified terms

## Future Extensions

Later term-metaprogramming milestones can add:

- `term_eqo` / `term_neqo` names for `==/2` and `\==/2`
- full standard term ordering predicates such as `@</2`
- `numbervars`-style variable naming
- richer goal construction once callable terms can be safely lowered into
  relation calls

## Summary

LP15 gives Python callers the first Prolog-style metaprogramming surface over
terms. That makes the library feel much closer to Prolog even before language
syntax exists, and it also creates reusable machinery for the future parser and
VM layers.
