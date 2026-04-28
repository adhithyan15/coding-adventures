# PR24: Prolog VM Length Stdlib

## Summary

This batch adds finite `length/2` support to the Prolog-on-Logic-VM path by
introducing `lengtho` in `logic-stdlib` and adapting parsed Prolog
`length(List, Length)` calls to that relation.

`length/2` is one of the most common list predicates in everyday Prolog. Adding
it after the first list-stdlib bridge makes the VM path much more usable for
real examples without requiring the parser or VM to grow a separate evaluator.

## Goals

- add `lengtho(list, length)` to the host-language relational standard library
- count proper finite lists
- validate a proper list against a known non-negative integer length
- create a fresh list skeleton when the length is a known non-negative integer
- adapt Prolog `length/2` in `prolog-loader`
- prove `length/2` works through parsed source, loader adaptation, VM
  compilation, and VM execution

## Semantics

The first implementation is deliberately finite and conservative:

- `lengtho([a, b, c], N)` yields `N = 3`
- `lengtho(List, 2)` creates a two-cell list skeleton that can unify with later
  goals
- improper lists fail
- negative or non-integer lengths fail
- `lengtho(List, N)` with both sides unknown fails instead of enumerating an
  infinite stream of list lengths

The final case is a conscious boundary. Full open-ended generation belongs with
future CLP(FD) and fair-search work, not with the first practical list-length
bridge.

## Example

```prolog
?- member(Item, [tea, cake]),
   append([Item], [jam], Combined),
   reverse(Combined, Reversed),
   length(Reversed, Count),
   length(Pair, 2),
   Pair = [left, right].
```

Expected VM answers:

```prolog
Item = tea,
Combined = [tea, jam],
Reversed = [jam, tea],
Count = 2,
Pair = [left, right].

Item = cake,
Combined = [cake, jam],
Reversed = [jam, cake],
Count = 2,
Pair = [left, right].
```

## Non-goals

- CLP(FD)-backed length domains
- fair infinite enumeration for `length(List, N)` with both arguments unknown
- full SWI `library(lists)` parity
