# Changelog

## Unreleased

- **Breaking:** `ct_select_bytes` returns
  `Result<Vec<u8>, SelectLengthMismatch>` instead of panicking when the slices
  differ in length. CT01 names this error `CT_SELECT_LENGTH_MISMATCH`
  (`SelectLengthMismatch::ID`); a value lets callers recover and lets the
  conformance adapter read the id without catching an unwind. The check still
  happens before either input is read. No crate in the repository called it.
- Run the shared CT01 corpus (`code/specs/fixtures/ct-compare-v1/cases.tfv`,
  21 cases) as the `ct01_corpus` test on the L0 bootstrap harness (TF01):
  results, the semantic error, fresh selection output and logical work counts.
  Four hand-made mutants (inverted `eq`, `select` ignoring `choice`, a 32-bit
  `eq_u64`, a truncating length mismatch) each fail it.

- Harden equal-length slice and fixed-array comparison so every loop iteration
  retains volatile reads of both inputs and the accumulator plus a volatile
  accumulator write, preventing the optimizer from eliding later work after an
  earlier difference.
- Add a release-optimized LLVM-IR regression gate that verifies every loop
  cycle crosses the volatile equality operations and every loop exit remains
  independent of their values.
- Scope the guarantee to compiler-observable full scans; hardware side-channel
  resistance remains explicitly out of scope.

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
