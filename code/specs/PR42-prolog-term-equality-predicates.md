# PR42: Prolog Term Equality Predicates

## Goal

Make the library-backed Prolog path understand the core term equality family
that real Prolog programs use before they reach more advanced constraint
predicates.

This batch focuses on executable behavior through the existing Logic VM path,
not on delayed constraints.

## Predicate Semantics

- `=/2` performs normal logic unification and may bind variables.
- `\=/2` succeeds only when the two terms cannot unify in the current state.
- `==/2` succeeds when the reified terms are strictly identical without adding
  bindings.
- `\==/2` succeeds when the reified terms are not strictly identical without
  adding bindings.

`dif/2` remains intentionally out of scope for this batch because full Prolog
`dif/2` is a delayed disequality constraint, not just negated unification.

## Implementation Shape

- `logic-builtins` exposes `not_same_termo(left, right)` as the strict
  non-identity counterpart to `same_termo(left, right)`.
- `prolog-loader` adapts parsed Prolog equality calls into executable
  engine/builtin goals.
- `prolog-vm-compiler` relies on the existing adapter path so source-level
  queries and compiled VM programs share the same behavior.

## Acceptance Tests

- Library tests prove strict non-identity does not bind either side.
- Loader tests prove parsed Prolog equality and disequality calls adapt into
  executable goals.
- VM stress tests prove successful and failing equality queries work end to end
  from Prolog source through parser, loader, compiler, and VM execution.
