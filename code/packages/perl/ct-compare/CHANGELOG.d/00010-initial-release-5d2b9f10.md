## 0.1.0

- Initial Perl lane of CT01: `ct_eq`, `ct_eq_fixed`, `ct_select_bytes`, `ct_eq_u64` and the
  bit-returning `ct_eq_bit`, `ct_eq_fixed_bit`, `ct_eq_u64_bit`.
- Integer selection bit (0 or 1); `feature bitwise` keeps `^ | &` numeric.
- u64 difference folded with a fixed shift sequence (unary minus on a UV above
  2**63 is a float in Perl).
- Tests replay the complete CT01 corpus and add exhaustive position, byte-value,
  length-mismatch, 64-bit-flip and validation checks (Devel::Cover 100%).
