# PR46: Prolog Arithmetic Runtime Errors

## Goal

Move the Prolog source-to-VM path closer to real Prolog behavior by raising
structured runtime errors for arithmetic evaluation failures.

## Motivation

The Python library predicates such as `iso(...)` and `lto(...)` are relational
helpers. They intentionally fail when arithmetic inputs are too open or not
numeric enough, which is convenient for Python-first logic programs.

Source-level Prolog arithmetic is different. Predicates such as `is/2`, `</2`,
and `=:=/2` are evaluable arithmetic predicates. They should not silently fail
when an expression contains an unbound variable, a non-evaluable term, or a
zero divisor. They should raise a Prolog runtime error that callers can catch.

## Design

- Keep existing library arithmetic helpers backward compatible.
- Add strict Prolog arithmetic goal helpers for loader/VM adapters.
- Add structured exception classes:
  - `PrologInstantiationError`
  - `PrologTypeError`
  - `PrologEvaluationError`
- Route parsed Prolog arithmetic predicates through the strict helpers.
- Let runtime errors propagate through the VM query APIs.

## Covered Predicates

- `is/2`
- `=:=/2`
- `=\=/2`
- `</2`
- `=</2`
- `>/2`
- `>=/2`

## Acceptance Tests

- `X is Y + 1` raises `PrologInstantiationError`.
- `X is tea + 1` raises `PrologTypeError`.
- `X is 1 / 0` raises `PrologEvaluationError` with `zero_divisor`.
- Arithmetic comparisons raise the same structured errors for invalid
  expressions.
- Existing Python library arithmetic helpers continue to fail logically instead
  of raising.
