# PR41: Module-Aware Apply Closures

This batch makes the higher-order apply-family predicates respect linked module
imports before they reach the Logic VM.

## Scope

- Rewrite apply-family closure arguments during project linking using the same
  resolver path already used by `call/N`.
- Cover `maplist/2..5`, `convlist/3`, `include/3`, `exclude/3`,
  `partition/4`, `foldl/4..7`, and `scanl/4..7`.
- Preserve explicit `Module:Closure` handling and imported predicate handling.
- Prove the behavior through loader tests and full parser -> loader -> compiler
  -> Logic VM stress coverage.

## Example

```prolog
:- module(apply_helpers, [increment/2, push/3]).
increment(1, 2).
increment(2, 3).
push(Item, Acc, [Item|Acc]).
```

```prolog
:- module(app, []).
:- use_module(apply_helpers, [increment/2, push/3]).

?- maplist(increment, [1,2], Ys),
   scanl(push, [a,b], [], States).
```

The linker qualifies `increment/2` and `push/3` as imported closures before the
query is adapted into executable builtin goals.

## Non-goals

- Full `meta_predicate/1` declaration parsing and validation.
- Automatic import of arbitrary unexported predicates.
- New apply-family predicates beyond the currently supported surface.
