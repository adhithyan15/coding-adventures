# PR26: Prolog VM Nth Stdlib

## Summary

This batch adds finite positional list access to the Prolog-on-Logic-VM path.
The implementation lives in `logic-stdlib` as `nth0o` and `nth1o`, and
`prolog-loader` adapts parsed Prolog `nth0/3` and `nth1/3` calls onto those
relations.

## Goals

- add `nth0o(index, list, element)` for zero-based proper-list indexing
- add `nth1o(index, list, element)` for one-based proper-list indexing
- enumerate index-element pairs when the list is proper and finite
- adapt Prolog `nth0/3` and `nth1/3`
- verify the predicates compose through parsed source, loader adaptation, VM
  compilation, and VM execution

## Semantics

The first implementation is finite:

- the list argument must be a proper finite list
- known non-negative integer indexes select one element
- open index variables enumerate all valid index-element pairs
- out-of-range, negative, non-integer, open-list, and improper-list cases fail

## Example

```prolog
?- reverse([tea, jam], Reversed),
   nth0(1, Reversed, ZeroBased),
   nth1(2, Reversed, OneBased).
```

Expected VM answer:

```prolog
Reversed = [jam, tea],
ZeroBased = tea,
OneBased = tea.
```

## Non-goals

- open-tail list generation
- `nth0/4` or `nth1/4`
- CLP(FD)-backed index domains
