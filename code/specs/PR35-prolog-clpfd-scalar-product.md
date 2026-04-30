# PR35: Prolog CLP(FD) Scalar Product

## Goal

Support the common Prolog CLP(FD) weighted-sum constraint:

```prolog
scalar_product(Coefficients, Variables, #=, Total)
```

This extends the Logic VM's CLP(FD) modeling layer beyond plain sums so users
can express knapsack, scheduling cost, and linear scoring problems directly.

## Scope

- Add a reusable `fd_scalar_producto(coeffs, vars, total)` library relation.
- Support Python sequences and proper logic lists for both coefficient and
  variable inputs.
- Add finite-domain evaluation and interval pruning for weighted sums,
  including negative coefficients.
- Adapt Prolog `scalar_product/4` when the relation operator is `#=`.
- Preserve unsupported operators for ordinary predicate resolution or later
  adapter support.
- Verify the direct library path, Prolog loader path, and compiled VM path.

## Example

```prolog
?- [X,Y] ins 0..4,
   scalar_product([2,3], [X,Y], #=, 12),
   X #< Y,
   labeling([], [X,Y]).
```

Expected answer:

```prolog
X = 0, Y = 4.
```

## Follow-Up Work

- Add the non-equality scalar-product operators when matching VM primitives
  exist for `#\=`, `#<`, `#=<`, `#>`, and `#>=`.
- Add more global constraints such as `element/3` and `global_cardinality/2`.
- Consider stronger linear-constraint propagation once larger examples need it.
