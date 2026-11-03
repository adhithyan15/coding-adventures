# go/ct-compare

Constant-time byte equality, byte selection, and unsigned 64-bit equality helpers.

## Bit-returning equality

The selection bit is an integer in this language, because a boolean cannot
become one without a branch. To feed an equality result into selection, use
the bit-returning variants, which return exactly 1 or 0 with no conditional:

```
CTSelectBytes(a, b, CTEqBit(x, y))
```

`CTEqFixedBit` and `CTEqU64Bit` are the fixed-width and 64-bit forms. Never convert the
boolean `CTEq` result into a bit with an `if`.

