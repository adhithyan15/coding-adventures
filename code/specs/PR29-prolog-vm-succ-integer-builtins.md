# PR29: Prolog VM Succ Integer Builtins

## Summary

This batch adds the next finite integer builtins to the Prolog-on-Logic-VM
path. The host library exposes `integero(term)` and `succo(predecessor,
successor)` in `logic-builtins`, while `prolog-loader` adapts parsed
`integer/1` and `succ/2` calls before VM compilation.

## Goals

- add `integero(term)` for non-bool integer type checks
- add `succo(predecessor, successor)` for non-negative integer successor
  relations
- adapt parsed Prolog `integer/1` and `succ/2`
- verify parser -> loader -> VM execution

## Semantics

- `integero(term)` succeeds when the reified term is an integer and not a bool
- `succo(left, right)` succeeds when `left >= 0` and `right = left + 1`
- if one side of `succo/2` is a concrete valid integer and the other is open,
  the open side is generated
- negative predecessors, zero successors without a predecessor, non-integers,
  and two-open-variable calls fail

## Example

```prolog
?- between(1, 3, Count),
   succ(Count, Next),
   integer(Next).
```

Expected:

```prolog
Count = 1, Next = 2 ;
Count = 2, Next = 3 ;
Count = 3, Next = 4.
```

## Non-Goals

- delayed CLP(FD)-style successor constraints
- unbounded Peano-style generation
- exception-throwing ISO error modes
