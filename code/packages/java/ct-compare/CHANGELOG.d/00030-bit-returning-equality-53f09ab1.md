## 0.3.0

- **Bit-returning equality variants** `ctEqBit`, `ctEqFixedBit` and `ctEqU64Bit`. Each
  returns an `int` that is exactly 1 (equal) or 0 (different, including a
  public length mismatch), computed from the difference accumulator with
  arithmetic only (CT01, "How a lane receives the selection bit"). The
  selection bit is an integer in this lane, and turning a boolean equality
  result into one would need a branch on a secret, so callers now write
  `ctSelectBytes(a, b, ctEqBit(x, y))` with no conditional anywhere.
- Tests check every one-byte pair against the boolean variants, a difference
  at every bit of a 64-byte input, 64-bit edge values and single-bit
  differences, and composition with selection.
