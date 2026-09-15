## 0.1.8 — 2026-08-13 — baseline regenerated after the f32 NaN payload bug was fixed (WASM13)

No code changes in this crate — `wasm-execution` 0.6.6 fixed a real bug
(every f32 value silently lost its exact NaN bit pattern passing through
the interpreter's typed operand stack, an arithmetic-cast round-trip
artifact, not just an issue for values an opcode computed on) surfaced
running this crate's own harness against the real testsuite. Baseline
regenerated: `assert_return` 13495/13518 (99.8%) → 13512/13518 (100.0%,
+17). Verified via a full per-file diff that exactly 4 files changed
(`conversions.wast`, `float_literals.wast`, `float_misc.wast`,
`local_tee.wast`), every one going from some real fails to zero — no
regressions, no partial fixes. See `wasm-execution`'s own `0.6.6`
changelog entry for the full bug writeup.

