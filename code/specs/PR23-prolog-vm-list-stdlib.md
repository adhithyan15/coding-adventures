# PR23: Prolog VM List Stdlib

## Summary

This batch connects common Prolog list predicates to the existing relational
standard library when source is loaded for the Logic VM path.

The Python library already has reusable list relations such as `membero`,
`appendo`, `selecto`, `permuteo`, and `reverseo`. This layer makes ordinary
Prolog source call those relations through familiar predicate names.

## Goals

- adapt `is_list/1` to `listo`
- adapt `last/2` to `lasto`
- adapt `member/2` to `membero`
- adapt `permutation/2` to `permuteo`
- adapt `reverse/2` to `reverseo`
- adapt `append/3` to `appendo`
- adapt `select/3` to `selecto`
- verify the adapted predicates compose through compiled VM source queries

## Example

```prolog
?- member(Item, [tea, cake]),
   append([Item], [jam], Combined),
   reverse(Combined, Reversed),
   select(Item, [tea, cake, jam], Remainder).
```

Expected VM answers:

```prolog
Item = tea,
Combined = [tea, jam],
Reversed = [jam, tea],
Remainder = [cake, jam].

Item = cake,
Combined = [cake, jam],
Reversed = [jam, cake],
Remainder = [tea, jam].
```

## Non-goals

- full SWI `library(lists)` parity
- `length/2`, which still needs a good finite-domain/natural-number story for
  relational generation
- higher-order list predicates such as `maplist/N`
