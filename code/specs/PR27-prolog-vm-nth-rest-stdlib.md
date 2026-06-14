# PR27: Prolog VM Nth Rest Stdlib

## Summary

This batch extends finite positional list access with Prolog `nth0/4` and
`nth1/4`. The implementation lives in `logic-stdlib` as `nth0_resto` and
`nth1_resto`, while `prolog-loader` adapts parsed source calls into those
relations before Logic VM execution.

## Goals

- add `nth0_resto(index, list, element, rest)`
- add `nth1_resto(index, list, element, rest)`
- support known integer indexes over proper finite lists
- enumerate index-element-rest triples for proper finite lists
- adapt Prolog `nth0/4` and `nth1/4`
- verify parser -> loader -> VM execution

## Semantics

- list arguments must be proper and finite
- known indexes select one element and the remaining list
- open index variables enumerate all valid positions
- out-of-range, negative, non-integer, open-list, and improper-list cases fail

## Example

```prolog
?- reverse([tea, jam], Reversed),
   nth0(1, Reversed, ZeroBased, ZeroRest),
   nth1(2, Reversed, OneBased, OneRest).
```

Expected:

```prolog
Reversed = [jam, tea],
ZeroBased = tea,
ZeroRest = [jam],
OneBased = tea,
OneRest = [jam].
```

## Non-Goals

- open-tail list generation
- CLP(FD)-backed index domains
- non-finite list search
