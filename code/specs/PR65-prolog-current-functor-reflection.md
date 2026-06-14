# PR65: Prolog Functor Reflection

## Status

Implemented for the Python builtin, loader, and Logic VM compiler path.

## Goal

Add `current_functor/2` so source programs running on the Logic VM can inspect
the functor indicators visible from the current program and builtin layer.

## Behavior

- `current_functor(Name, Arity)` enumerates visible source predicate indicators.
- Compound functors found inside visible source and dynamic clauses are
  reflected by name and arity.
- Atoms found inside visible clauses are reflected as `Atom/0`, matching the
  `functor/3` view of atomic terms.
- Builtin predicate indicators are included for metaprogramming.
- Dynamic assertions are reflected through the branch-local dynamic database.
- Source-level `current_functor/2` is adapted into the shared builtin and runs
  through compiled Logic VM queries.

## Scope

This is an environment-level reflection predicate, not a global process-wide
atom/functor table. It intentionally reports what is observable from the current
Logic VM program and proof branch.
