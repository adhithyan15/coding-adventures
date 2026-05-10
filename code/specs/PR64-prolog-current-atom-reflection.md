# PR64: Prolog Atom Reflection

## Status

Implemented for the Python builtin, loader, and Logic VM compiler path.

## Goal

Add `current_atom/1` so Prolog source running on the Logic VM can discover
atoms visible from the loaded program and builtin environment.

## Behavior

- `current_atom(Atom)` enumerates atoms from visible static and dynamic clauses.
- Predicate names and compound functors are included as atoms.
- Builtin predicate names, Prolog flag names, flag values, and predicate
  property atoms are included for metaprogramming queries.
- Dynamic assertions are reflected through the branch-local dynamic database.
- Source-level `current_atom/1` is adapted into the shared builtin and runs
  through compiled Logic VM queries.

## Scope

This is not a global process-wide atom table yet. It intentionally reflects the
atoms observable from the current Logic VM program and proof branch, which is
the useful layer for embeddable Prolog execution and deterministic tests.
