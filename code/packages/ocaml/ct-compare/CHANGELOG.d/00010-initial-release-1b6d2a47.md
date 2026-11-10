## 0.1.0

- Initial OCaml lane of CT01: `ct_eq`, `ct_eq_fixed`, `ct_select_bytes`, `ct_eq_u64` and the
  bit-returning `ct_eq_bit`, `ct_eq_fixed_bit`, `ct_eq_u64_bit` over `string` and `int64`.
- `ct_select_bytes` returns `(string, error) result` (`Select_length_mismatch`,
  `Invalid_choice`); the selection bit is an `int` that must be 0 or 1.
- Alcotest suite replays the complete CT01 corpus (vendored copy plus a drift test against
  the canonical file) and adds exhaustive position, byte-value, length, 64-bit-flip and
  invalid-choice checks.
