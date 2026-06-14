# PR40: Prolog Apply-Family Predicates

This batch extends the callable-term-backed higher-order list layer toward the
common SWI-style `library(apply)` surface.

## Scope

- Extend `maplisto` and parsed `maplist/N` through arity 5.
- Extend `foldlo` and parsed `foldl/N` through arity 7 so closures can consume
  up to four same-length input lists before the accumulator pair.
- Add `convlisto/3` and parsed `convlist/3` for map-plus-filter workflows.
- Add `scanlo/4..7` and parsed `scanl/4..7` for collecting intermediate
  accumulator states.
- Prove direct builtin behavior, loader adaptation, and full parser -> loader
  -> compiler -> Logic VM execution.

## Examples

```prolog
join4(A, B, C, joined(A, B, C)).
convert(1, one).
convert(3, three).
pair_push(Left, Right, Acc, [pair(Left, Right)|Acc]).
push(Item, Acc, [Item|Acc]).

?- maplist(join4, [a,b], [x,y], [1,2], Joined),
   convlist(convert, [1,2,3], Converted),
   foldl(pair_push, [a,b], [x,y], [], Folded),
   scanl(push, [a,b], [], Scanned).
```

Expected bindings:

```prolog
Joined = [joined(a,x,1), joined(b,y,2)],
Converted = [one, three],
Folded = [pair(b,y), pair(a,x)],
Scanned = [[a], [b,a]].
```

## Non-goals

- Full `library(apply)` coverage for every dialect-specific helper.
- Declarative generation of arbitrary unknown input lists.
- `meta_predicate/1` declaration analysis.
