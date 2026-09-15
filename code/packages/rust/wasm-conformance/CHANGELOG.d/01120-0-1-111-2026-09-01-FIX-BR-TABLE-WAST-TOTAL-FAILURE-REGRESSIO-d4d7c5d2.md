## 0.1.111 — 2026-09-01 — fix: `br_table.wast` total-failure regression (found during corpus prioritization pass)

A fresh prioritization pass over the vendored testsuite (pinned at SHA
`28864811cf03bdbf880733786148feaba339582d`) turned up a real, previously
un-investigated regression baked into the checked-in `testsuite-status.
json` baseline: `br_table.wast` — a foundational MVP-level control-flow
file, no GC-proposal syntax involved — was TOTALLY failing (`module
0/1`, `assert_return 0/161`), all cascading from the one module failing
structural validation. This had shipped silently because the baseline
mechanism only detects CHANGES from a prior baseline, not absolute
correctness — a regression that lands and then never changes again is
invisible to it indefinitely.

Root cause: two independent bugs, both in how a table declared with a
CONCRETE function-reference type (`(table $t (ref null $t) (elem $tf))`
— function-references proposal, but an entirely ordinary MVP-adjacent
construct) interacted with `table.get` and `br_table`'s multi-target
type check. Full technical writeup in `wasm-validator`'s own CHANGELOG
(the fix landed there and in `wasm-wast-parser`/`wasm-types`); this
entry covers the corpus-wide verification.

### Corpus impact

Baseline regenerated (`cargo run --release --bin wasm_conformance_report
-- --write-baseline`) and diffed PROGRAMMATICALLY against the pre-fix
`testsuite-status.json` (comparing the `files` dict, keyed by filename,
across all 257 parsed files) — not spot-checked. Exactly **one** file's
tally changed:

| file | directive | before | after |
|---|---|---|---|
| `br_table.wast` | `module` | 0/1 pass (1 fail) | 1/1 pass |
| `br_table.wast` | `assert_return` | 0/161 pass (161 fail) | 161/161 pass |
| `br_table.wast` | everything else (`assert_invalid`, etc.) | unchanged | unchanged |

Every other file in the corpus that ALSO declares a table with a
concrete/GC-flavored reftype (`table.wast`, `elem.wast`, `type-
subtyping.wast`, `table-sub.wast`, `ref.wast`, `linking.wast`, `ref_eq.
wast`, `i31.wast`, `ref_is_null.wast`, `ref_cast.wast`, `ref_test.wast`)
was checked individually against its own before/after entry, not just
inferred from the unchanged aggregate: every one of them already hit an
EARLIER `NotYetSupported` gate (a whole-module feature-detection heuristic
that fires on OTHER unsupported constructs elsewhere in the same module)
before validation ever reached the code this fix touches, so neither bug
was reachable in any of them — confirming this fix is precisely scoped
to the one file it was meant to repair, with zero regressions and zero
missed collateral fixes elsewhere in the corpus.

Aggregate `module`/`assert_return` percentages both round to the same
displayed figure before and after (br_table.wast's own 1 module + 161
assert_return directives are a small fraction of the corpus's 1983/51779
totals), so the per-file diff above — not the aggregate line — is the
real evidence this fix worked and didn't regress anything.

