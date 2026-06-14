# PR30: Prolog VM CLP(FD) Adapter

## Summary

This batch bridges parsed callable CLP(FD) forms into the existing
`logic-builtins` finite-domain engine. It intentionally supports callable
predicate forms that parse today while leaving infix operator parsing for a
later grammar/operator-table batch.

## Goals

- adapt `in/2` to `fd_ino`
- adapt `ins/2` over proper variable lists
- adapt `#=/2`, `#\=/2`, `#</2`, `#=</2`, `#>/2`, and `#>=/2`
- lower simple `#=(Result, +(Left, Right))`, `-(...)`, and `*(...)` forms
- adapt `all_different/1`, `all_distinct/1`, `label/1`, and `labeling/2`
- verify parser-to-loader-to-VM execution

## Semantics

- domains are finite and concrete; list domains work today
- `ins/2` requires a proper finite target list
- `labeling/2` currently ignores options and labels its second argument
- `#=/2` falls back to FD equality when neither side is a supported arithmetic
  expression

## Example

```prolog
?- ins([X,Y], [1,2,3]),
   in(Z, [1,2,3,4,5,6]),
   #<(X,Y),
   #=(Z, +(X,Y)),
   all_different([X,Y]),
   labeling([], [X,Y,Z]).
```

Expected:

```prolog
X = 1, Y = 2, Z = 3 ;
X = 1, Y = 3, Z = 4 ;
X = 2, Y = 3, Z = 5.
```

## Non-Goals

- infix CLP(FD) operator parsing, such as `X #= Y + 1`
- `1..4` range syntax parsing
- labeling option semantics
- full recursive arithmetic-expression lowering
