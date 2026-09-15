## 0.1.6 — 2026-08-13 — baseline regenerated after inline-import shorthand was fixed (WASM02)

No code changes in this crate — `wasm-wast-parser` 0.1.4 fixed a real bug
(`func`/`table`/`memory`/`global` **inline-import shorthand** wasn't
recognized, and fixing it exposed a deeper pre-existing indexing bug once
a module could combine an import with a same-kind real definition)
surfaced running this crate's own harness against the real testsuite.
Baseline regenerated: `func_ptrs.wast` goes from a full parse failure to
100% passing every directive kind it has; `assert_return` 12219/12238
(99.8%) → 12235/12254 (99.8%, +16). Verified via a full per-file diff that
`func_ptrs.wast` is the only file whose tally changed anywhere in the
corpus. See `wasm-wast-parser`'s own `0.1.4` changelog entry for the full
bug writeup.

