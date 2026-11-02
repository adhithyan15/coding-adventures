# Changelog

All notable changes to this package will be documented in this file.

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
