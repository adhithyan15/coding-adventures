## 0.1.3 — 2026-08-13 — baseline regenerated after a local-index bug fix (WASM14)

No code changes in this crate — `wasm-wast-parser` 0.1.2 fixed a real bug
(a declared local aliasing parameter index 0 when a function references
its signature only via `(type $sig)`) surfaced by running this crate's
own harness against the real testsuite. Baseline regenerated:
`assert_return` 12169/12238 (99.4%) → 12171/12238 (99.5%).

