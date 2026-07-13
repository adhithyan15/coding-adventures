# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `hash-functions` crate: the non-cryptographic
  "DT17" hash family implemented from scratch.
- Free functions: `hf_fnv1a_32`, `hf_fnv1a_64`, `hf_djb2`,
  `hf_polynomial_rolling` / `_with_params`, `hf_murmur3_32` / `_with_seed`,
  `hf_siphash_2_4`, and the string helpers `hf_hash_str_fnv1a_32` /
  `hf_hash_str_siphash`. Named constants (`HF_FNV32_*`, `HF_FNV64_*`,
  `HF_DJB2_OFFSET_BASIS`, `HF_POLYNOMIAL_ROLLING_DEFAULT_*`).
- `HfHashFunction` tagged struct with `hf_new_*` constructors, `hf_hash`, and
  `hf_output_bits` — the Rust `HashFunction` trait folded into one value.
- Analysis helpers: `hf_avalanche_score` (bit-flip sensitivity via a
  caller-supplied byte source; Rust's `getrandom` OS entropy has no pure-ISO
  equivalent) and `hf_distribution_test` (chi-square uniformity).
- Rust's `u128` intermediate in polynomial rolling is replaced by an exact,
  overflow-safe `mulmod`/`addmod`, matching results for any 64-bit modulus with
  no 128-bit type.
- 48 checks mirroring the crate's known-answer vectors, run under every ISO C
  compiler via the shared `iso-harness`; also clean under ASan + UBSan.
