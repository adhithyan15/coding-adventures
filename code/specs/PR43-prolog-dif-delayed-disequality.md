# PR43: Prolog `dif/2` Delayed Disequality

## Goal

Add the next equality-family predicate needed for practical Prolog programs:
`dif/2`.

Unlike `\=/2`, which answers whether two terms cannot unify right now, `dif/2`
is a delayed disequality constraint. It may succeed while terms are still open,
then reject any later binding that would make those terms equal.

## Semantics

- `dif(X, tea)` succeeds while `X` is unbound and records a pending constraint.
- `dif(X, tea), X = cake` succeeds.
- `dif(X, tea), X = tea` fails when the later binding violates the constraint.
- `dif(Left, Right)` also works for structured terms as they become
  instantiated.

## Implementation Shape

- `logic-builtins` exposes `difo(left, right)` as the library-facing delayed
  disequality goal.
- `prolog-loader` maps parsed `dif/2` calls to `difo/2`.
- `prolog-vm-compiler` inherits the behavior through the existing adapter path,
  so source queries, compiled instruction programs, and stateful runtimes share
  one implementation.

## Non-Goals

- This batch does not replace `\=/2`; that remains immediate
  non-unifiability.
- This batch does not add residual constraint rendering to named answers.

## Acceptance Tests

- Library tests prove `difo/2` can succeed before variables are bound and later
  reject equal bindings.
- Loader tests prove parsed `dif/2` adapts into delayed disequality.
- VM stress tests prove `dif/2` works end to end through parser, loader,
  compiler, and Logic VM execution.
