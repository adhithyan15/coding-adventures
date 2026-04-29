# PR34: Prolog CLP(FD) Sum Global

## Goal

Support the common Prolog CLP(FD) global constraint:

```prolog
sum(Vars, #=, Total)
```

This keeps the Prolog frontend aligned with the existing Logic VM
`fd_sumo/2` primitive instead of requiring users to spell larger additions as
nested infix `#=/2` expressions.

## Scope

- Parse SWI-style `sum(List, #=, Total)` through the existing callable syntax.
- Adapt the parsed `sum/3` goal to the library `fd_sumo(List, Total)` relation.
- Leave unsupported `sum/3` operators unchanged for ordinary predicate
  resolution or future adapter support.
- Verify the loader path and the compiled VM query path with an end-to-end
  CLP(FD) example.

## Example

```prolog
?- [X,Y,Z] ins 1..4,
   sum([X,Y,Z], #=, 6),
   X #< Y,
   Y #< Z,
   labeling([], [X,Y,Z]).
```

Expected answer:

```prolog
X = 1, Y = 2, Z = 3.
```

## Follow-Up Work

- Add additional `sum/3` relation operators if the VM gains compatible
  primitives for `#\=`, `#<`, `#=<`, `#>`, and `#>=`.
- Add more CLP(FD) global constraints such as `scalar_product/4`,
  `global_cardinality/2`, and `element/3`.
