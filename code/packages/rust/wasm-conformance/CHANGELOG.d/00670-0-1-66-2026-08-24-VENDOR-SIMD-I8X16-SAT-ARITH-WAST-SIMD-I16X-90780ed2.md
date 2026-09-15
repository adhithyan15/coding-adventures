## 0.1.66 — 2026-08-24 — vendor simd_i8x16_sat_arith.wast/simd_i16x8_sat_arith.wast: SIMD widen PR33 (task #214-216)

### Added

- Vendored `simd_i8x16_sat_arith.wast` and `simd_i16x8_sat_arith.wast`
  (both new files, not re-fetches of already-vendored ones) at the
  existing pinned commit `28864811cf03bdbf880733786148feaba339582d`.
  This PR implements the 8 opcodes needed for the saturating integer
  add/sub family: `i8x16.add_sat_s` (`0x6F`), `i8x16.add_sat_u` (`0x70`),
  `i8x16.sub_sat_s` (`0x72`), `i8x16.sub_sat_u` (`0x73`),
  `i16x8.add_sat_s` (`0x8F`), `i16x8.add_sat_u` (`0x90`),
  `i16x8.sub_sat_s` (`0x92`), `i16x8.sub_sat_u` (`0x93`), implemented in
  `wasm-opcodes`/`wasm-execution`/`wasm-validator`/`wasm-wast-parser` as
  part of the same PR.
- `simd_i8x16_sat_arith.wast`: 188/188 `assert_return`, 12/12
  `assert_invalid`, 12/12 `assert_malformed`, 2/2 modules, ALL 100%
  passing on the first baseline regen after implementation.
  `simd_i16x8_sat_arith.wast`: 204/204 `assert_return`, 12/12
  `assert_invalid`, 4/4 `assert_malformed`, 2/2 modules, ALL 100%
  passing -- including every one of the corpus's own boundary-value and
  underflow/overflow-saturation vectors (the unsigned-underflow-
  saturates-to-zero direction, this family's classic bug spot, passes
  cleanly at both lane widths). Aggregate `assert_return` rose from
  33619/33636 to 34011/34028 (+392 pass, +392 gradeable, exactly these
  two files' combined `assert_return` count, `fail` unchanged at 17);
  `assert_invalid` rose from 1814/1814 to 1838/1838 (+24, exactly these
  two files' combined count, still 100.0% of gradeable directives);
  `assert_malformed` rose from 319/319 to 335/335 (+16, exactly these
  two files' combined count, still 100.0% of gradeable directives);
  `module` pass count rose from 1182/1183 to 1186/1187 (+4, one per
  module across both files, all passing). The pre-existing, unrelated
  baseline failures (17 `assert_return`, 1 `module`, 1
  `assert_unlinkable`, 2 `register` -- present before this PR and
  tracked separately) are byte-for-byte unchanged, confirming zero
  regressions.
- `fetch_testsuite.py`'s `TESTSUITE_FILES` gains both new filenames at
  the end of the SIMD block, following the established chronological/
  PR-commented convention.
- `NOTICE` updated with the fetch-header extension line and a new
  paragraph account for this PR.
- Baseline regenerated via `--write-baseline`; `corpus_matches_the_
  committed_baseline` passes cleanly against the new baseline.

