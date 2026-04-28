# PR22: Prolog VM If-Then Runtime

## Summary

This batch connects parsed Prolog if-then control constructs to the executable
Logic VM path.

The logic builtin layer already had `iftheno(...)` and `ifthenelseo(...)`.
This layer teaches the Prolog loader adapter to map source-level `->/2` and
`(If -> Then ; Else)` into those builtins, then verifies the compiled VM path
preserves committed condition semantics.

## Goals

- adapt `If -> Then` into `iftheno(...)`
- adapt `If -> Then ; Else` into `ifthenelseo(...)`
- commit to the first proof of `If`
- keep `Then` branch backtracking after the committed condition proof
- run `Else` from the original state when `If` fails
- preserve rule-local variable freshening for callable condition and branch
  terms

## Example

```prolog
candidate(first).
candidate(second).

chosen(Value) :-
    (candidate(Candidate) -> Value = Candidate ; Value = none).

?- chosen(Chosen).
```

The VM should return only:

```prolog
Chosen = first
```

The condition commits to the first `candidate/1` proof, so `second` is not
explored and the `else` branch is not run.

## Non-goals

- soft-cut `*->/2`
- catch/throw exception control
- full ISO top-level tracing/debugging semantics
