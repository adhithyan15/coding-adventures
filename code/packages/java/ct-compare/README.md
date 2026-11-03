# ct-compare

Constant-time byte equality, fixed-size equality alias, mask-based byte
selection, and branch-free 64-bit equality helpers for Java.

## Bit-returning equality

The selection bit is an integer in this language, because a boolean cannot
become one without a branch. To feed an equality result into selection, use
the bit-returning variants, which return exactly 1 or 0 with no conditional:

```
ctSelectBytes(a, b, ctEqBit(x, y))
```

`ctEqFixedBit` and `ctEqU64Bit` are the fixed-width and 64-bit forms. Never convert the
boolean `ctEq` result into a bit with an `if`.

