# PR25: Prolog VM Sort Stdlib

## Summary

This batch adds finite `sort/2` and `msort/2` support to the Prolog-on-Logic-VM
path. The implementation lives in `logic-stdlib` as `sorto` and `msorto`, and
`prolog-loader` adapts parsed Prolog calls onto those shared relations.

## Goals

- add `sorto(list, sorted)` for duplicate-free finite sorting
- add `msorto(list, sorted)` for finite sorting that preserves duplicates
- adapt Prolog `sort/2` and `msort/2`
- verify the predicates compose through parsed source, loader adaptation, VM
  compilation, and VM execution

## Semantics

The first implementation is intentionally finite:

- the input must be a proper finite list
- open tails and improper lists fail
- `sort/2` removes duplicate terms before sorting
- `msort/2` keeps duplicate terms
- sorting follows the Prolog-inspired standard term order already used by term
  comparison and `setof/3`

## Example

```prolog
?- member(Item, [tea, cake]),
   sort([Item, jam, Item], UniqueSorted),
   msort([Item, jam, Item], Sorted).
```

Expected VM answers:

```prolog
Item = tea,
UniqueSorted = [jam, tea],
Sorted = [jam, tea, tea].

Item = cake,
UniqueSorted = [cake, jam],
Sorted = [cake, cake, jam].
```

## Non-goals

- `keysort/2`
- custom comparison predicates
- sorting open list tails
