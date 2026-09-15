## 0.1.16 — 2026-08-15 — baseline regen: call.wast even/odd now pass (WASM10)

### Changed

- Baseline regen following `wasm-execution` 0.7.0 (WASM10 — `call_function`
  now runs on a dedicated thread with a re-bisected, much higher
  `MAX_CALL_DEPTH`): `call.wast`'s `assert_return` moves from `pass: 67,
  fail: 2` to `pass: 69, fail: 0` — the `even(100)`/`odd(200)` mutual-
  recursion cases that previously needed more than the old 80-depth
  ceiling now complete. Confirmed via a full baseline diff that this is
  the ONLY file affected; zero regressions elsewhere.

