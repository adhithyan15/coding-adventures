# PR61: Prolog Unifiability Predicates

## Goal

Expose explicit source-level unifiability predicates on top of the Logic VM.
This gives Prolog programs a way to ask how two terms can unify without
necessarily mutating the caller's bindings, and a named finite-unification
predicate that makes the engine's occurs-check behavior visible.

## Scope

The builtin and loader layers expose:

```text
unifiable/3
unify_with_occurs_check/2
```

The VM path supports both through parser, loader adapter, compiler, and runtime
execution.

## Semantics

This batch implements deterministic finite unifiability:

- `unify_with_occurs_check(Left, Right)` unifies `Left` and `Right` using the
  engine's finite-term occurs check.
- Self-referential bindings such as `X = box(X)` fail under
  `unify_with_occurs_check/2`.
- `unifiable(Left, Right, Unifier)` succeeds when the two terms can unify and
  unifies `Unifier` with a proper list of `Var = Value` equations.
- `unifiable/3` does not bind the source terms. It computes the first
  deterministic unifier in an isolated successor state, reifies each
  first-occurrence source variable under that state, and reports only variables
  that gained bindings.
- Terms that cannot unify make `unifiable/3` fail.

## Verification

- `logic-builtins` tests cover finite unification, occurs-check rejection,
  non-binding unifier extraction, and non-unifiable failure.
- `prolog-loader` tests cover source-level adaptation for both predicates.
- `prolog-vm-compiler` stress coverage runs both predicates end-to-end through
  parser, loader, compiler, VM, and named-answer extraction.
