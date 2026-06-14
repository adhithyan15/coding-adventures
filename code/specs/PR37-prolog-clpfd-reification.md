# PR37: Prolog CLP(FD) Reification

This batch adds the first truth-valued CLP(FD) layer shared by the Python
library API, parsed SWI-style Prolog source, and the Logic VM path.

## Scope

- Add `fd_reify_relationo(left, op, right, truth)` as the library primitive for
  finite-domain comparisons that produce or consume a `0..1` truth variable.
- Add boolean FD connectives:
  - `fd_bool_ando(left, right, result)`
  - `fd_bool_oro(left, right, result)`
  - `fd_bool_noto(value, result)`
  - `fd_bool_implieso(left, right, result)`
  - `fd_bool_equivo(left, right, result)`
- Add SWI operator defaults for `#<==>`, `#==>`, `#/\\`, `#\\/`, and prefix
  `#\\`.
- Lower parsed Prolog reification syntax onto the same builtins.

## Example

```prolog
?- [X,Y,Z] ins 1..3,
   (X #< Y) #<==> A,
   (Y #< Z) #<==> B,
   (A #/\ B) #<==> Chain,
   Chain #= 1,
   labeling([], [X,Y,Z,A,B,Chain]).
```

The expected answer is the single strictly increasing chain:

```prolog
X = 1, Y = 2, Z = 3, A = 1, B = 1, Chain = 1.
```

## Non-goals

- Full attributed-variable CLP(FD) propagation.
- Reified global constraints such as `all_distinct/1 #<==> B`.
- Optimization/search directives beyond the existing labeling option subset.
