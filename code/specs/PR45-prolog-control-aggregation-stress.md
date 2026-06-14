# PR45: Prolog VM Control And Aggregation Stress

## Goal

Prove that the Prolog VM path can run practical control and aggregation
programs from source syntax, not only isolated library-level builtins.

## Scope

This batch fixes rule-local variable freshening for embedded callable goals in
control builtins and adds end-to-end stress coverage for:

- negation-as-failure with `\+/1`
- deterministic probing with `once/1`
- universal checks with `forall/2`
- collection predicates `findall/3`, `bagof/3`, and `setof/3`

## Motivation

The underlying logic builtins and loader adapters already expose these
predicates. What was missing was one compact source-level regression that shows
they compose through the complete stack:

```text
SWI-style source -> parser -> loader adapter -> Prolog VM compiler -> Logic VM
```

This matters because real Prolog programs frequently combine control and
aggregation in one query. A parser-only or builtin-only test can miss integration
failures in operator handling, callable-goal adaptation, variable visibility, or
VM query answer projection.

The new stress case exposed one of those integration failures: `onceo(...)`,
`noto(...)`, `forallo(...)`, `findallo(...)`, `bagofo(...)`, and `setofo(...)`
captured embedded goal objects too early when used inside rules, so rule
freshening could leave body variables disconnected from head variables. The fix
mirrors the existing `iftheno(...)` strategy by passing representable callable
goals as native-goal arguments and lowering them again from the active reified
state.

## Acceptance Tests

- `\+/1` succeeds for non-provable goals and fails for provable goals.
- `once/1` commits to the first proof of a generator.
- `forall/2` succeeds when every generated proof satisfies the test.
- `findall/3` preserves all answers in proof order.
- `bagof/3` preserves grouped answers and fails only when empty.
- `setof/3` deduplicates and sorts collected answers.
