# Changelog

## 0.1.0 — 2026-04-20

- Initial release.
- `ct_eq(a: &[u8], b: &[u8]) -> bool`: XOR-accumulator equality on
  byte slices. Same number of iterations regardless of where (or
  whether) bytes differ.
- `ct_eq_fixed::<N>(&[u8; N], &[u8; N]) -> bool`: type-safe
  constant-length variant for MAC-tag / key comparisons.
- `ct_select_bytes(a, b, choice) -> Vec<u8>`: branchless conditional
  select using `0u8.wrapping_sub(choice as u8)` mask and
  `b ^ ((a ^ b) & mask)` per byte.
- `ct_eq_u64(a, b) -> bool`: constant-time 64-bit equality for
  counters and lease IDs.
- All public entry points are `#[inline(never)]` and wrap the
  accumulator in `core::hint::black_box` before the final comparison.
- `#![deny(unsafe_code)]` — zero unsafe blocks.
- 18 unit tests: every-single-bit-position flip coverage for both
  `ct_eq` and `ct_eq_u64`, length-mismatch handling, empty slices,
  high-bit detection, `ct_select_bytes` branchless select correctness
  across the full 0..=255 byte range.
- Built for the D18 Chief-of-Staff Vault crypto stack and for any
  MAC / AEAD tag / password-hash comparison.
