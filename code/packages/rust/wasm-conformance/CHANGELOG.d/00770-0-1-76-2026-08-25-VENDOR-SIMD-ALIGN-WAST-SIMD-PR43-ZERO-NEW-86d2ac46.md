## 0.1.76 — 2026-08-25 — vendor simd_align.wast (SIMD PR43, zero new opcodes)

### Added

- Vendored `simd_align.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering `align=` hint
  validation for `v128.load`/`v128.store` plus the `load_splat`/
  `load_extend` families landed in PR15/PR39-42. Adds ZERO new opcodes
  — every instruction this file exercises was already implemented.
  Added to `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated
  with full provenance and per-file directive-kind counts.
- The file passes 100% on `module` (46/46) and `assert_return` (8/8),
  the directives that only exercise already-implemented opcode
  execution. `assert_invalid` grades 0/12 (all `NotYetSupported`) and
  `assert_malformed` grades 12/34 pass with 22 `NotYetSupported`: this
  repo's SIMD memarg decode path in `wasm-validator` never checks an
  over-large `align=` against each v128 opcode's natural alignment (the
  scalar `iNN.load`/`iNN.store` arm does, via `memory_op_shape`/
  `max_align_for`, but the shared v128 memarg arm doesn't), and
  `wasm-wast-parser`'s `parse_memarg` never validates that `align=N` is
  a power of two. Both gaps are pre-existing and shared with the class
  already tracked as `NotYetSupported` in the plain (non-SIMD)
  `align.wast` vendored earlier — this file surfaces the same known gap
  for the v128 opcode family, tracked for a dedicated follow-up, not a
  new regression.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `module` rose from 1214/1215 to 1260/1261 (+46 pass, 67
  `NotYetSupported` unchanged); `assert_return` rose from 44392/44409 to
  44400/44417 (+8 pass, 545 `NotYetSupported` unchanged); `assert_invalid`
  stayed at 2013/2013 pass with `NotYetSupported` rising from 76 to 88
  (+12, exactly this file's own gap); `assert_malformed` rose from
  513/513 to 525/525 pass (+12) with `NotYetSupported` rising from 516
  to 538 (+22, exactly this file's own gap). No other already-vendored
  file's stats changed — zero regressions.

