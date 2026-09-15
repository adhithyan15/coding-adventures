## 0.1.51 — 2026-08-19 — vendor simd_splat.wast; baseline regen (task #165-167)

### Changed

- Baseline regen: vendored `simd_splat.wast` -- `i8x16.splat`/
  `i16x8.splat`/`i64x2.splat`, widening lane-width coverage of the
  already-implemented `i32x4.splat`, see `wasm-opcodes`'s own
  CHANGELOG entry. Vendoring this file first surfaced a real,
  unrelated `wasm-wast-parser` bug (a `_` digit separator inside a
  `nan:0x<payload>` literal made the whole script fail to parse) --
  fixed as part of this same PR, see that crate's own CHANGELOG.
- Once the file parses, its own upstream structure bundles all 6 lane
  widths' splat exports (including `f32x4.splat`/`f64x2.splat`,
  neither implemented -- this crate has zero float-lane SIMD support
  at all) into the SAME modules the new integer-lane directives
  depend on, so every one of this file's 158 `assert_return`
  directives grades `NotYetSupported` until float-lane SIMD support
  lands in a future PR. `assert_invalid` (22/22) and `assert_malformed`
  (1/1) DO pass today, since those directives don't require the
  shared modules to build. Vendored now anyway since it's the real
  upstream file, the 3 new opcodes are independently verified via
  dedicated unit tests in the meantime, and this file will
  automatically flip to real credit with zero further changes once
  float-lane support exists -- see `tests/fixtures/testsuite/NOTICE`
  for the full breakdown. Aggregate `assert_return` stays 24040/24057
  (pass/gradeable, unchanged) but its `NotYetSupported` count rose
  from 552 to 710 (+158, exactly this file's own `assert_return`
  count); `assert_invalid` rose from 1681/1681 to 1703/1703 (+22) and
  `assert_malformed` from 274/274 to 275/275 (+1).

