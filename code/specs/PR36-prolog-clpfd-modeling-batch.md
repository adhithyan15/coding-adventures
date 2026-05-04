# PR36: Prolog CLP(FD) Modeling Batch

## Goal

Move from one-predicate CLP(FD) slices to a larger modeling batch that makes
the Prolog-on-Logic-VM path more useful for real constraint programs.

This batch focuses on common linear and indexing constraints:

```prolog
sum(Vars, #=<, Total)
scalar_product(Coeffs, Vars, #>, Total)
element(Index, Vars, Value)
```

## Scope

- Add relation-aware sum constraints through `fd_sum_relationo/3`.
- Add relation-aware scalar products through `fd_scalar_product_relationo/4`.
- Keep `fd_sumo/2` and `fd_scalar_producto/3` as equality conveniences.
- Add `fd_elemento/3` as a 1-based CLP(FD) element/indexing constraint.
- Adapt Prolog `sum/3`, `scalar_product/4`, and `element/3` through the loader.
- Cover direct library usage, Prolog loader adaptation, and compiled VM stress
  execution in one batch.

## Supported Relations

The relation-aware linear constraints accept:

- `#=` / `eq`
- `#\=` / `neq`
- `#<` / `lt`
- `#=<` / `le`
- `#>` / `gt`
- `#>=` / `ge`

The Prolog adapter intentionally recognizes the Prolog spellings. The Python
library accepts both Prolog spellings and compact internal names.

## Example

```prolog
?- [I,X,Y,Z] ins 1..4,
   I #= 2,
   element(I, [X,Y,Z], 4),
   sum([X,Y,Z], #=<, 8),
   scalar_product([2,1,1], [X,Y,Z], #>, 8),
   all_different([X,Y,Z]),
   labeling([], [I,X,Y,Z]).
```

Expected answers:

```prolog
I = 2, X = 1, Y = 4, Z = 3 ;
I = 2, X = 2, Y = 4, Z = 1 ;
I = 2, X = 3, Y = 4, Z = 1.
```

## Follow-Up Work

- Add CLP(FD) reification and boolean connectives.
- Add stronger propagation for `#\=` and larger linear constraints.
- Add `global_cardinality/2` and tuple/table constraints in later modeling
  batches.
