# PR28: Prolog VM Between Stdlib

## Summary

This batch adds finite `between/3` support to the Prolog-on-Logic-VM path. The
host library exposes it as `betweeno(low, high, value)` in `logic-builtins`, and
`prolog-loader` adapts parsed `between/3` calls before VM compilation.

## Goals

- add `betweeno(low, high, value)` to the builtin goal layer
- generate inclusive integer ranges when `value` is open
- validate a bound integer value against finite bounds
- adapt parsed Prolog `between/3`
- verify parser -> loader -> VM execution inside collection queries

## Semantics

- `low` and `high` must be concrete non-bool integers
- `value` may be an open logic variable or a concrete non-bool integer
- generation is deterministic ascending order from `low` through `high`
- descending ranges, non-integer bounds, and non-integer values fail

## Example

```prolog
?- between(1, 4, Number), Number > 2.
```

Expected:

```prolog
Number = 3 ;
Number = 4.
```

## Non-Goals

- symbolic or unbounded integer ranges
- CLP(FD)-backed delayed bounds
- support for SWI-Prolog's `inf`/`infinite` bounds
