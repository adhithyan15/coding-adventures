## 0.1.63 — 2026-08-24 — vendor simd_f32x4_cmp.wast: SIMD widen PR30, now the biggest directive-count win in the campaign (task #205-207)

### Added

- Vendored `simd_f32x4_cmp.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  6 opcodes needed to close `f32x4`'s comparison family: `f32x4.eq`
  (`0x41`), `f32x4.ne` (`0x42`), `f32x4.lt` (`0x43`), `f32x4.gt`
  (`0x44`), `f32x4.le` (`0x45`), `f32x4.ge` (`0x46`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- **Now the single biggest directive-count win in this campaign,
  surpassing PR29's `simd_f32x4_arith.wast`:** 2581 `assert_return` +
  18 `assert_invalid` + 6 `assert_malformed` = 2605 total gradeable
  directives across 2 modules, ALL 100% passing on the first baseline
  regen after implementation (0 `NotYetSupported`, 0 failures).
  Aggregate `assert_return` rose from 26573/26590 to 29154/29171
  (+2581); `assert_invalid` rose from 1762/1762 to 1780/1780 (+18);
  `assert_malformed` rose from 307/307 to 313/313 (+6); `module` pass
  count rose from 1175 to 1177 (+2). The pre-existing, unrelated
  baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged by
  this PR. No other already-vendored file's stats changed. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

