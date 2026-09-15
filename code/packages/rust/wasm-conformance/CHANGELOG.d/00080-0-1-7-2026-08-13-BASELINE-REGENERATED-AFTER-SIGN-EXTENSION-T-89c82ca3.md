## 0.1.7 — 2026-08-13 — baseline regenerated after sign-extension/trunc_sat opcodes were added (WASM03)

No code changes in this crate — `wasm-opcodes` 0.2.1, `wasm-wast-parser`
0.1.5, and `wasm-execution` 0.6.5 added the sign-extension and trunc_sat
opcode families (plus fixed a real, pre-existing boundary bug in the
trapping `trunc_*` handlers those additions exposed) surfaced running this
crate's own harness against the real testsuite. Baseline regenerated:
`i32.wast`/`i64.wast`/`conversions.wast` go from full parse failures to
parsing and running (98.9%+ passing across them); `assert_return`
12235/12254 (99.8%) → 13495/13518 (99.8%, +1260 across the newly-parseable
files); `assert_trap` 331/331 (100%) → 418/418 (100%). Verified via a full
per-file diff that these 3 newly-parseable files are the only ones whose
tally changed anywhere in the corpus. See `wasm-execution`'s own `0.6.5`
changelog entry for the full bug writeup, including the 4 remaining
`conversions.wast` fails (an unrelated, already-tracked NaN-payload gap,
WASM13).

