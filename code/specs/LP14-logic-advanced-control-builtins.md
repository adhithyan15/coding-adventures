# LP14 - Logic Advanced Control Builtins

## Overview

LP11 introduced the first Prolog-style control helpers: `callo`, `onceo`, and
`noto`. LP12 added evaluative arithmetic, and LP13 added solution collection.
The next functionality-level gap is richer control flow that lets library users
write programs that look more like practical Prolog without needing Prolog
syntax yet.

LP14 extends the Python `logic-builtins` package with truth, failure,
if-then-style control, and universal quantification.

## Design Goal

Add a small advanced-control layer that is honest about the current solver's
capabilities:

- expose explicit truth and failure predicates
- support committed-condition if-then and if-then-else goals
- support `forall`-style checks over generated solutions
- document why full Prolog cut is deferred

These are ordinary Python functions that return `logic-engine` goal
expressions. They are not syntax and they do not require a parser.

## Package

Update:

```text
code/packages/python/logic-builtins
```

The package version should move from `0.3.0` to `0.4.0`.

## Public API

Add:

```python
trueo()
failo()
iftheno(condition, then_goal)
ifthenelseo(condition, then_goal, else_goal)
forallo(generator, test)
```

The `o` suffix keeps the package convention that public predicates return
logic goals.

## Semantics

### `trueo()`

Succeed once without changing the current state.

This is the library spelling of Prolog's `true/0` and mirrors
`logic_engine.succeed()`.

### `failo()`

Fail without yielding any successor states.

This is the library spelling of Prolog's `fail/0` and mirrors
`logic_engine.fail()`.

### `iftheno(condition, then_goal)`

Run `condition` from the current state. If `condition` has at least one proof,
commit to the first condition proof and run `then_goal` from that proof state.
If `condition` has no proof, fail.

This is equivalent to Prolog's `Condition -> Then` form without an else branch.
The condition is committed to one proof; the then branch may still produce
multiple solutions.

### `ifthenelseo(condition, then_goal, else_goal)`

Run `condition` from the current state.

- If `condition` has at least one proof, commit to its first proof and run
  `then_goal` from that proof state.
- If `condition` has no proofs, run `else_goal` from the original state.

The else branch must not see bindings from a failed condition attempt.

### `forallo(generator, test)`

Run `generator` from the current state. For every generated proof state, run
`test` from that proof state. `forallo` succeeds once from the original state if
every generated proof satisfies `test` at least once.

Important details:

- `forallo` succeeds vacuously when `generator` has no proofs.
- bindings produced by `generator` and `test` do not leak to the outer state
- if any generated proof fails `test`, the whole `forallo` goal fails

This is the library spelling of Prolog's `forall/2`, implemented as an
operational check over the proof stream rather than as classical universal
quantification.

## Cut Boundary

LP14 deliberately does not add `cuto()`.

Prolog cut (`!/0`) prunes choicepoints created to the left of the cut inside a
clause. The current solver exposes generator-based proof streams and native
goals, but it does not yet represent scoped choicepoints or a "commit" signal
that can prune surrounding disjunctions and clause alternatives. A fake `cuto`
that merely succeeds once would be actively misleading.

A future milestone can add real cut by extending the solver protocol with
scoped choicepoint pruning. Until then, `onceo`, `iftheno`, and
`ifthenelseo` provide honest committed-choice building blocks.

## Error Model

Passing a non-goal object where a goal is required is host-language API misuse
and should raise `TypeError`.

Logical failure remains logical:

- failed conditions make `iftheno` fail
- failed conditions choose the else branch in `ifthenelseo`
- failed tests make `forallo` fail

## Test Strategy

Required tests:

- `trueo` succeeds once and preserves the current state
- `failo` fails
- `iftheno` commits to the first condition proof
- `iftheno` allows the then branch to produce multiple solutions
- `iftheno` fails when the condition fails
- `ifthenelseo` chooses the then branch when the condition succeeds
- `ifthenelseo` chooses the else branch from the original state when the
  condition fails
- `forallo` succeeds when every generated proof satisfies the test
- `forallo` fails when any generated proof fails the test
- `forallo` succeeds vacuously for an empty generator
- advanced control helpers reject non-goals

## Future Extensions

Later control milestones can add:

- scoped solver support for real Prolog cut
- soft-cut variants
- cleanup/finalization control such as `setup_call_cleanup`
- bounded search helpers such as `limit` and `call_nth`

## Summary

LP14 gives the library enough control flow to express more realistic Prolog
programs while keeping the implementation aligned with the current
backtracking engine. It adds useful committed-choice and universal-check
building blocks without pretending that the solver already has full cut.
