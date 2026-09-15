## 0.1.36 — 2026-08-18 — vendor simd_i32x4_arith.wast + simd_i32x4_cmp.wast; baseline regen (task #113-117)

### Changed

- Baseline regen: vendored `simd_i32x4_arith.wast`/`simd_i32x4_cmp.wast`,
  previously deferred as needing more `i32x4` SIMD opcode coverage than
  the original 5-opcode first slice provided (see `wasm-opcodes`'s own
  CHANGELOG entry for the 12 newly-added opcodes). `simd_i32x4_arith.wast`:
  100% pass on every gradeable directive (2/2 modules, 181/181
  assert_return, 11/11 assert_invalid). `simd_i32x4_cmp.wast`: 100% pass
  on every gradeable directive (420/420 assert_return, 30/30
  assert_invalid); its one `NotYetSupported` module and 13
  `NotYetSupported` assert_return directives exercise
  `i32x4.trunc_sat_f32x4_s`/`f32x4.const` float-to-int boundary values,
  genuinely out of scope for this i32x4-only widening pass. Aggregate
  `assert_return` rose from 20586/20603 to 21187/21204.

