# PR33: Prolog CLP(FD) Labeling Options

## Summary

This batch keeps moving the finite-domain layer from "engine exists" toward
"Prolog programs feel natural" by honoring a first subset of `labeling/2`
options. The prior adapter ignored the option list and always used the default
engine order.

## Goals

- add `labeling_optionso(options, vars)` to the logic builtin layer
- support variable-selection options `ff` and `leftmost`
- support value-order options `up` and `down`
- accept both Python sequences and proper logic lists for option lists
- route parsed Prolog `labeling/2` through the option-aware builtin
- verify loader and full Prolog VM execution for `labeling([down], Vars)`

## Example

```prolog
?- X in 1..3,
   labeling([down], [X]).
```

Expected answers:

```prolog
X = 3 ;
X = 2 ;
X = 1.
```

## Non-Goals

- full SWI labeling strategy coverage
- optimization options such as `min(Expr)` and `max(Expr)`
- branching options beyond accepting `enum` and `step` as current no-ops
