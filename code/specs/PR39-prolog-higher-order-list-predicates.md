# PR39: Prolog Higher-Order List Predicates

This batch adds the first higher-order list predicates on top of the callable
term runtime introduced for `call/N`.

## Scope

- Add library-level `maplisto/2..4`, `includeo/3`, `excludeo/3`,
  `partitiono/4`, and `foldlo/4`.
- Execute predicate closures through `calltermo(...)` so atoms, compounds, and
  module-qualified callable terms share the same meta-call path.
- Adapt parsed Prolog `maplist/2..4`, `include/3`, `exclude/3`, `partition/4`,
  and `foldl/4` into the library builtin layer.
- Prove the behavior with direct logic builtin tests, Prolog loader tests, and
  a Logic VM stress test.

## Examples

```prolog
small(1).
small(2).
increment(1, 2).
increment(2, 3).
increment(3, 4).
push(Item, Acc, [Item|Acc]).

?- maplist(increment, [1,2,3], Ys),
   include(small, [1,2,3], Small),
   exclude(small, [1,2,3], Big),
   partition(small, [1,2,3], Yes, No),
   foldl(push, [a,b,c], [], Stack).
```

Expected bindings:

```prolog
Ys = [2,3,4],
Small = [1,2],
Big = [3],
Yes = [1,2],
No = [3],
Stack = [c,b,a].
```

## Non-goals

- Full `library(apply)` coverage such as `convlist/N`, `scanl/N`, or
  `foldl/5+`.
- `meta_predicate/1` declaration semantics.
- Relational generation of arbitrary source lists for `include/3`, `exclude/3`,
  `partition/4`, and `foldl/4`.
