# PR31: Prolog CLP(FD) Infix Syntax

## Summary

This batch makes the SWI-Prolog frontend accept the natural CLP(FD) syntax that
users expect to write on top of the Logic VM finite-domain layer. The previous
batch adapted callable forms such as `#=(Z, +(X,Y))`; this one parses operator
forms such as `Z #= X + Y`.

## Goals

- tokenize `..` as one SWI symbolic atom rather than two statement dots
- add SWI operator defaults for CLP(FD) comparisons, `in/2`, `ins/2`, and
  range terms
- keep the dialect grammar file aware of CLP(FD) infix forms
- verify parser lowering for `[X,Y] ins 1..4`, `X in 1..4`, and arithmetic
  constraints such as `Z #= X + Y`
- run an end-to-end Prolog VM stress query using natural CLP(FD) syntax

## Example

```prolog
?- [X,Y] ins 1..3,
   Z in 1..6,
   X #< Y,
   Z #= X + Y,
   all_different([X,Y]),
   labeling([], [X,Y,Z]).
```

Expected answers:

```prolog
X = 1, Y = 2, Z = 3 ;
X = 1, Y = 3, Z = 4 ;
X = 2, Y = 3, Z = 5.
```

## Non-Goals

- parsing every CLP(FD) reification/connective operator
- implementing labeling option semantics
- importing SWI's full `library(clpfd)` module surface
