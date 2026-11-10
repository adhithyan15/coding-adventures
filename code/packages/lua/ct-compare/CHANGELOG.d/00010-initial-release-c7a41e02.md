## 0.1.0

- Initial Lua lane of CT01: `ct_eq`, `ct_eq_fixed`, `ct_select_bytes`, `ct_eq_u64` and the
  bit-returning `ct_eq_bit`, `ct_eq_fixed_bit`, `ct_eq_u64_bit`.
- Integer selection bit (0 or 1), validated without a secret-dependent branch.
- Tests replay the complete CT01 corpus and add exhaustive position, byte-value,
  length-mismatch, 64-bit-flip and validation checks.
