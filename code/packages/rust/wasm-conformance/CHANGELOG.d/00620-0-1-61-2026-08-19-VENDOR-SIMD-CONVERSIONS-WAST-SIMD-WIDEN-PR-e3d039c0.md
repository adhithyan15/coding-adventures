## 0.1.61 — 2026-08-19 — vendor simd_conversions.wast: SIMD widen PR28, campaign complete (task #199-201)

### Added

- Vendored `simd_conversions.wast` (new file, not a re-fetch of an
  already-vendored one) at the existing pinned commit
  `28864811cf03bdbf880733786148feaba339582d`. This is the THIRD and
  FINAL PR of a 3-PR sequence (PR26 "extend", 8 opcodes; PR27
  "narrow", 4 opcodes; this PR's "promote/demote/convert_low", 4
  opcodes) needed to land all 16 opcodes this single upstream file's
  two modules bundle together -- unlike every earlier SIMD file in
  this campaign, `simd_conversions.wast` could NOT be partially
  vendored, since both modules export functions exercising opcodes
  from all three PRs at once. This PR adds the last 4:
  `f32x4.demote_f64x2_zero` (`0x5E`), `f64x2.promote_low_f32x4`
  (`0x5F`), `f64x2.convert_low_i32x4_s` (`0xFE`),
  `f64x2.convert_low_i32x4_u` (`0xFF`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser`
  as part of the same PR.
- **100% pass on EVERY directive** (2/2 modules, 232/232
  `assert_return`, 18/18 `assert_invalid`, 30/30 `assert_malformed` --
  280 directives total), zero `NotYetSupported` anywhere -- the first
  real integration test exercising opcodes from all three PRs
  (extend/narrow/promote-demote-convert_low) together in one corpus
  file, not just three separate opcode-inventory claims. Aggregate
  `assert_return` rose from 24538/24555 to 24770/24787 (+232);
  `assert_invalid` rose from 1728/1728 to 1746/1746 (+18);
  `assert_malformed` rose from 277/277 to 307/307 (+30); `module` pass
  count rose from 1170 to 1172 (+2). No other already-vendored file's
  stats changed. See `tests/fixtures/testsuite/NOTICE` for the full
  breakdown.
- **Real parser gap found and fixed while vendoring, not just opcode
  work:** before this PR, `simd_conversions.wast` failed to PARSE AT
  ALL (a hard script-level parse error, not a per-directive grading
  gap) because two of its `assert_return` directives use `(v128.const
  f64x2 nan:canonical nan:canonical)` as an EXPECTED value --
  `nan:canonical`/`nan:arithmetic` NaN-class tokens inside a v128
  literal's individual lanes, something `wasm-wast-parser` had no
  representation for at all (only whole-scalar `f32.const`/`f64.const`
  NaN classes were supported). Fixed in `wasm-wast-parser` (see that
  crate's own CHANGELOG): a new `Expected::V128F32x4`/`V128F64x2`
  per-lane representation, gated so a `v128.const` with no NaN-class
  lanes still uses the original byte-exact path unchanged (zero
  regression risk across every already-vendored file using
  `v128.const`, confirmed by the full baseline diff above showing no
  other file's stats moved).

