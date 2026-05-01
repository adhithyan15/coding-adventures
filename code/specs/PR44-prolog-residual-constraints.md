# PR44: Prolog VM Residual Constraints

## Goal

Expose delayed constraints that remain after a Prolog query succeeds so Python
callers can inspect complete Prolog-style answers, not only variable bindings.

## Motivation

`dif/2` can succeed before both sides are fully known by leaving a pending
disequality constraint in the Logic VM state. The VM already preserves those
constraints, but the named Prolog answer helpers previously converted proof
states into raw output values and discarded the residual constraint store.

That made queries such as `?- dif(X, tea).` look like a plain unbound `X`
answer, hiding the important condition `X \= tea`.

## Design

- Keep raw value helpers such as `run_compiled_prolog_query(...)` unchanged.
- Preserve proof states in named answer helpers so bindings and residuals come
  from the same VM solution.
- Add `PrologAnswer.residual_constraints`, a tuple of reified `Disequality`
  values.
- Reify residual constraint terms through the answer state's substitution before
  exposing them.
- Apply the same answer shape to compiled source queries and stateful ad-hoc
  runtime queries.

## Example

```python
answers = runtime.query("dif(X, tea).")
answer = answers[0]

assert "X" in answer.as_dict()
assert answer.residual_constraints
```

## Follow-Ups

- Add residual visibility for future constraint families beyond disequality.
- Add a higher-level pretty-printer once the Prolog top-level display layer
  exists.
