# PR32: Prolog CLP(FD) Nested Additive Arithmetic

## Summary

This batch expands the Prolog loader's CLP(FD) adapter from single binary
arithmetic equality expressions to nested additive expressions. It lets natural
source queries like `Z #= X + Y + 1` run through the same Logic VM finite-domain
path as simpler `Z #= X + Y` constraints.

## Goals

- flatten nested `+/2` equality expressions into `fd_sumo(...)`
- support integer offsets in nested additive expressions, including forms such
  as `Z #= X + Y - 1`
- keep simple binary `+`, `-`, and `*` equality lowering on the existing
  `fd_addo`, `fd_subo`, and `fd_mulo` constraints
- verify parser-to-loader and full Prolog VM execution

## Example

```prolog
?- [X,Y] ins 1..3,
   Z in 4..6,
   X #< Y,
   Z #= X + Y + 1,
   labeling([], [X,Y,Z]).
```

Expected answers:

```prolog
X = 1, Y = 2, Z = 4 ;
X = 1, Y = 3, Z = 5 ;
X = 2, Y = 3, Z = 6.
```

## Non-Goals

- hidden temporary-variable domains for arbitrary nested multiplication
- symbolic linear coefficients such as `Z #= X - Y`
- full CLP(FD) expression compilation for every SWI arithmetic form
