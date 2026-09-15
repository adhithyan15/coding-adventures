## 0.1.52 — 2026-08-19 — baseline regen: f32x4/f64x2.splat unblocks simd_splat.wast (task #168-170)

### Changed

- Baseline regen, no new file vendored: `f32x4.splat`/`f64x2.splat`
  (see `wasm-opcodes`'s own CHANGELOG entry) unblock 115 of the 158
  `assert_return` directives in the already-vendored `simd_splat.wast`
  (task #165-167) that were stuck `NotYetSupported` -- exactly as
  predicted when that file was vendored. 3 of its 4 modules (pure
  splat, splat-into-store/load, splat-into-control-construct) needed
  ONLY the two new float splats to build; the remaining module (43
  directives) stays `NotYetSupported`, since it additionally needs
  `extract_lane`/`replace_lane` for multiple lane widths and
  `i8x16.swizzle`, none implemented yet. Aggregate `assert_return`
  rose from 24040/24057 to 24155/24172 (+115 pass, +115 gradeable,
  matching the predicted unblock count exactly); `module` pass count
  rose from 1152 to 1155 (+3) and its `NotYetSupported` count fell
  from 78 to 75. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.

