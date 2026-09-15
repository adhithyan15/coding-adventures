## 0.1.72 — 2026-08-25 — vendor simd_f32x4_rounding.wast/simd_f64x2_rounding.wast (SIMD widen PR39)

### Added

- Vendored `simd_f32x4_rounding.wast` and `simd_f64x2_rounding.wast`
  (pinned commit `28864811cf03bdbf880733786148feaba339582d`) covering
  the new `f32x4.ceil`/`floor`/`trunc`/`nearest` and `f64x2.ceil`/
  `floor`/`trunc`/`nearest` opcodes added in `wasm-opcodes`/
  `wasm-execution`/`wasm-validator`/`wasm-wast-parser`. Added to
  `TESTSUITE_FILES` in `fetch_testsuite.py`; NOTICE updated with full
  provenance and per-file directive-kind counts.
- Both files pass 100% of their directives: 1/1 `module`, 176/176
  `assert_return`, 8/8 `assert_invalid`, 16/16 `assert_malformed`, in
  EACH file (352/352, 16/16, 32/32, 2/2 combined).

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 43865/43882 to 44217/44234 (+352
  pass, `fail` unchanged at 17); `assert_invalid` rose from 1973/1973 to
  1989/1989 (+16, still 100%); `assert_malformed` rose from 465/465 to
  497/497 (+32, still 100%); `module` pass count rose from 1206/1207 to
  1208/1209 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

