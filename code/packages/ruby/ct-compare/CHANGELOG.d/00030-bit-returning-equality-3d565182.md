## 0.3.0

- **Bit-returning equality variants** `ct_eq_bit`, `ct_eq_fixed_bit` and `ct_eq_u64_bit`. Each
  returns an `Integer` that is exactly 1 (equal) or 0 (different, including a
  public length mismatch), computed from the difference accumulator with
  arithmetic only (CT01, "How a lane receives the selection bit"). The
  selection bit is an integer in this lane, and turning a boolean equality
  result into one would need a branch on a secret, so callers now write
  `ct_select_bytes(a, b, ct_eq_bit(x, y))` with no conditional anywhere.
- Tests check every one-byte pair against the boolean variants, a difference
  at every bit of a 64-byte input, 64-bit edge values and single-bit
  differences, and composition with selection.
