## 0.1.4 — 2026-08-13 — baseline regenerated after a branch double-pop bug fix (WASM11)

No code changes in this crate — `wasm-execution` 0.6.4 fixed a real bug
(a branch to an outer block double-popped `label_stack`, corrupting
control flow for any branch that unwound past one or more already-open
outer blocks) surfaced by running this crate's own harness against the
real testsuite's `switch.wast`. Baseline regenerated: `assert_return`
12171/12238 (99.4%) → 12215/12238 (99.8%).

