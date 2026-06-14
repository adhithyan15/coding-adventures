# PR47: Prolog Exception Control

## Goal

Add the first source-level Prolog exception-control layer on top of the Logic VM
path.

## Motivation

Real Prolog programs use `throw/1` and `catch/3` to separate exceptional control
flow from ordinary logical failure. PR46 introduced structured arithmetic
runtime errors, but callers still had no Prolog-level way to recover from them.

This batch connects those pieces:

- user code can throw arbitrary Prolog terms
- `catch/3` can recover when the catcher pattern unifies with the thrown ball
- structured runtime errors are represented as catchable `error(Formal, Context)`
  terms

## Design

- Add `PrologThrown` as the internal exception used by `throwo/1`.
- Add `throwo(ball)` for library-level exception throws.
- Add `catcho(goal, catcher, recovery)` for protected goal execution.
- Preserve the state at the throw point so recovery can observe bindings made
  before the exception.
- Rethrow non-matching exceptions.
- Translate structured `PrologRuntimeError` instances into catchable
  `error(..., logic_runtime)` terms.
- Adapt parsed Prolog `throw/1` and `catch/3` calls through `prolog-loader`.

## Covered Source Predicates

- `throw/1`
- `catch/3`

## Acceptance Tests

- `catch(throw(boom), boom, Recovered = true)` succeeds.
- Non-matching catchers rethrow the original exception.
- Recovery can observe bindings made before `throw/1`.
- Normal protected goal solutions pass through unchanged.
- Arithmetic runtime errors can be caught with `error(instantiation_error, _)`.
- Parsed Prolog source can run exception control end to end through the Logic VM.
