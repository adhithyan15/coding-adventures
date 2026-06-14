# PR58: Prolog Term Hash

## Goal

Add a practical structural hashing primitive to the Prolog-on-Logic-VM path.
This supports indexing, memoization, cache keys, and table-like user code
without requiring callers to round-trip terms through text.

## Scope

The builtin and loader layers expose:

```text
term_hash/2
term_hash/4
```

The VM path supports both through parser, loader adapter, compiler, and
runtime execution.

## Semantics

This batch implements deterministic, variant-aware structural hashes:

- `term_hash(Term, Hash)` unifies `Hash` with a non-negative integer derived
  from the reified term structure.
- Variant terms hash alike. For example, `pair(X, X)` and `pair(Y, Y)` produce
  the same hash.
- Variable-sharing shape is preserved. For example, `pair(X, X)` and
  `pair(X, Y)` produce different hashes.
- `term_hash(Term, Depth, Range, Hash)` limits traversal by `Depth` and
  returns a hash in `0 =< Hash < Range`.
- Invalid `Depth` or `Range` values fail cleanly for now: depth must be a
  non-negative integer and range must be a positive integer.
- Hashes are stable for this library implementation, but callers should treat
  them as indexing keys rather than portable Prolog dialect constants.

## Verification

- `logic-builtins` tests cover variant hashing, different variable-sharing
  shapes, bound hash validation, depth/range behavior, and invalid bounds.
- `prolog-loader` tests cover source-level `term_hash/2` and `term_hash/4`
  adaptation.
- `prolog-vm-compiler` stress coverage runs both forms end-to-end through
  parser, loader, compiler, VM, and named-answer extraction.
