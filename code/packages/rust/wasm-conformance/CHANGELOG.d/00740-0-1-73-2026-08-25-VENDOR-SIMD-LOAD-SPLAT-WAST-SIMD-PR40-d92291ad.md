## 0.1.73 — 2026-08-25 — vendor simd_load_splat.wast (SIMD PR40)

### Added

- Vendored `simd_load_splat.wast` (pinned commit
  `28864811cf03bdbf880733786148feaba339582d`) covering the new
  `v128.load8_splat`/`load16_splat`/`load32_splat`/`load64_splat`
  opcodes added in `wasm-opcodes`/`wasm-execution`/`wasm-validator`/
  `wasm-wast-parser`. Added to `TESTSUITE_FILES` in
  `fetch_testsuite.py`; NOTICE updated with full provenance and
  per-file directive-kind counts. First bite into the wider
  `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
  `store{8,16,32,64}_lane` memory-access family PR39 deferred --
  deliberately scoped to just this one file, the smallest/simplest of
  that family.
- The file passes 100% of its directives: 2/2 `module`, 80/80
  `assert_return`, 32/32 `assert_trap`, 8/8 `assert_invalid`, 4/4
  `assert_malformed`.

### Changed

- Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`).
  Aggregate `assert_return` rose from 44217/44234 to 44297/44314 (+80
  pass, `fail` unchanged at 17); `assert_trap` rose from 1430/1430 to
  1462/1462 (+32, still 100%); `assert_invalid` rose from 1989/1989 to
  1997/1997 (+8, still 100%); `assert_malformed` rose from 497/497 to
  501/501 (+4, still 100%); `module` pass count rose from 1208/1209 to
  1210/1211 (+2). The pre-existing, unrelated baseline failures (17
  `assert_return`, 1 `module`, 1 `assert_unlinkable`, 2 `register`) are
  byte-for-byte unchanged, confirming zero regressions.

