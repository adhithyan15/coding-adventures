## 0.1.118 — 2026-09-01 — baseline regen: active/declarative elem-segment drop fix closes the 4 failures 0.1.117 honestly reported

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-runtime` 0.6.29 fixed the exact bug the previous entry
(0.1.117) documented but deliberately did not fix: `instantiate()` never
marked an active or declarative element segment "dropped" once applied
(or, for a declarative segment, immediately, since it's never applied at
all) — see that crate's own CHANGELOG for the full spec-semantics
writeup and root cause. No code changes in this crate itself.

Programmatic per-file diff against the pre-fix baseline (Python,
comparing the `files` dict keyed by filename), across all 257 corpus
files: exactly 3 files changed, exactly the 4 directives 0.1.117
predicted, nothing else in the whole corpus moved:

| File | Directive kind | Before | After |
|---|---|---|---|
| `table_init.wast` | `assert_trap` | 583 pass / 1 fail | 584/584, 0 fail |
| `table_init64.wast` | `assert_trap` | 633 pass / 1 fail | 634/634, 0 fail |
| `elem.wast` | `assert_trap` | 4 pass / 2 fail / 1 NYS | 6 pass / 0 fail / 1 NYS |

Aggregate: `assert_trap` 4824→4828 pass, **0 fail** (was +4 new fail),
140 not-yet-supported unchanged. Every other directive kind (`module`,
`action`, `assert_return`, `assert_invalid`, `assert_malformed`,
`assert_unlinkable`, `assert_exception`, `assert_exhaustion`, `register`)
and every other of the 257 files is byte-for-byte unchanged from the
0.1.117 baseline — confirming this fix is exactly as scoped as intended,
with zero collateral impact on any other passive/declarative
element-segment interaction in the corpus.

