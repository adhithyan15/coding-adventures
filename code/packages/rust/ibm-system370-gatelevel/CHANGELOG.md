# Changelog

All notable changes to this package will be documented in this file.

## Unreleased

### Fixed

- Rust 1.100 clippy (`-D warnings`) drift in `src/lib.rs`: `needless_range_loop`: `settled_matches_oracle` compares GPRs, FPRs and memory with `iter().enumerate()`; the parsed-token `LIB_LOCK` in `tests/gate_integrity.rs` is refreshed for the token-level change (same behaviour); behaviour unchanged.
- `settled_matches_oracle` fails closed on an oracle whose memory is shorter than `MEMORY_BYTES`. The 1.100 clippy rewrite from an index loop to `iter().enumerate().take(MEMORY_BYTES)` had quietly stopped panicking on a short oracle and compared fewer bytes, so a truncated oracle could be reported equal. An explicit length check now returns `false`; the new unit test `truncated_oracle_memory_never_matches` covers it, and `LIB_LOCK` is re-pinned.

## [0.1.0] - 2026-10-26

### Added

- One embedded System/360 gate engine as the sole stable-Q authority for all
  8,389,403 persistent System/370 architectural bits.
- Independent gate-derived `MVCL`, `CLCL`, `CLM`, `STCM`, and `ICM` decode,
  address, length, overlap, comparison, mask, byte-routing, merge, and
  condition-code networks.
- System/370-owned public state, traces, results, typed transactional faults,
  checked allocation seams, and complete `step`/`run`/`execute` rollback.
- Fallible construction, state observation, memory reads, inherited execution,
  and functional-oracle trace creation, with an adjacent named injection seam
  for every real production allocation and atomic post-progress rollback.
- Twenty-one exhaustive transport, differential, truth-table, allocation, and
  compiler-checked parsed-structure evidence with 95.27% line coverage
  (825/866).
