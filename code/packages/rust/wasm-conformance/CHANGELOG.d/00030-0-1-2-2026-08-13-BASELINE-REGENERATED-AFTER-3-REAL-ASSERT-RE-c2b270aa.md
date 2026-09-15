## 0.1.2 — 2026-08-13 — baseline regenerated after 3 real assert_return bug fixes (WASM07)

No code changes in this crate — `wasm-execution` 0.6.3 and `wasm-runtime`
0.5.1 fixed 3 real bugs (an implicit function-body branch label, an
`instance.memory`/`tables` loss after any trapped call, and
`call_indirect` checking against the wrong index space) surfaced by
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 12030/12238 (98.3%) → 12169/12238 (99.4%).
See those crates' own changelogs for the full bug writeups.

