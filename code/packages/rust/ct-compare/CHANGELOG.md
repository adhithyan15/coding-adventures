# Changelog

## Unreleased

- Add the Warrant-derived evidence harness (VER04h): `tests/warrant_evidence.rs`
  runs `tests/generated/warrant_evidence.rs`, generated from
  `code/specs/ct-compare.warrant`, against the real public API: every example,
  bounded-exhaustive suites (120,409 `eq` cases; 65,536 `eq_fixed<1>` cases;
  142,129 `eq_u64` cases; ...), relation-biased property suites (4,096 cases
  each), fuzz suites (5,000 iterations each) and per-operation sabotage
  controls. `evidence/test-manifest.json` records what the run did.
  BUILD fails if the generated file drifts and checks each sabotage mode.

- Add equal-instruction-count microbenchmarks and deterministic cost budgeting
  (`tests/microbenchmarks.rs`, VER07 / VER07a / VER-P017):
  - Multi-scale $n \in \{16, 256, 4096\}$ sweeps for `ct_eq` with zero-allocation budget;
  - Equivalence class verification: identical, differing first, differing last, differing middle;
  - Compile-time fixed width comparison benchmarks (`ct_eq_fixed` for 16, 32, and 64 bytes);
  - Branch-free selection microbenchmarks (`ct_select_bytes`);
  - 64-bit integer comparison microbenchmarks (`ct_eq_u64`);
  - Hardware instruction count divergence assertions when PMU counters are live;
  - Negative sabotage test asserting divergence detection on naive short-circuiting comparison mutants.

- Keep negative adapter diagnostics within TF01's closed failure-ID vocabulary:
  rejected baselines and undetected injected defects report corpus-mismatch.
  Exercise the real failing TAP output with the always-pass sabotage mode.

- Run every CT01 shared corpus case through TF01's L0 harness, using the real
  fixed-array API, exact u64 values, native length-error mapping and selection
  ownership checks. Add five negative checks that catch broken adapter behavior.
- Generate TFV transport and const-generic dispatch from the independent JSON
  oracle; fail on drift and schedule the adapter on corpus/reference changes.
- Check all logical public-shape work fields without treating them as measured
  runtime counts or constant-time proofs. Keep the optimized equality CFG gate.
- Add the bootstrap harness as a development-only dependency and equivalent
  regeneration checks in BUILD and BUILD_windows; runtime dependencies stay empty.

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
