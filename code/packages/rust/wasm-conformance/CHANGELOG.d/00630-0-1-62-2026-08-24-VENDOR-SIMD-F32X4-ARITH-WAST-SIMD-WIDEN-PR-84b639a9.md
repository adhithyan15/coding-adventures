## 0.1.62 — 2026-08-24 — vendor simd_f32x4_arith.wast: SIMD widen PR29, biggest directive-count win in the campaign (task #202-204)

### Added

- Vendored `simd_f32x4_arith.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This PR implements the
  last 5 opcodes needed to close `f32x4`'s core arithmetic family:
  `f32x4.neg` (`0xE1`), `f32x4.sqrt` (`0xE3`), `f32x4.add` (`0xE4`),
  `f32x4.sub` (`0xE5`), `f32x4.div` (`0xE7`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- **The single biggest directive-count win in this campaign so far:**
  1803 `assert_return` + 16 `assert_invalid` = 1819 total gradeable
  directives across 3 modules, ALL 100% passing on the first baseline
  regen after implementation (0 `NotYetSupported`, 0 failures).
  Aggregate `assert_return` rose from 24770/24787 to 26573/26590
  (+1803); `assert_invalid` rose from 1746/1746 to 1762/1762 (+16);
  `module` pass count rose from 1172 to 1175 (+3). The pre-existing,
  unrelated baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register`) are byte-for-byte unchanged by
  this PR. No other already-vendored file's stats changed. See
  `tests/fixtures/testsuite/NOTICE` for the full breakdown.

