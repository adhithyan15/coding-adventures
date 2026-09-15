## 0.1.116 — 2026-09-01 — both remaining post-W35 gaps closed: `elem.wast` externref bug and `table.wast`'s oversized-minimum question

Resolves the two gaps `code/specs/W07-wasm-post-mvp-epics.md`'s
"Addendum (2026-09-01)" left open after W35 closed: `elem.wast`'s one
remaining `assert_return` failure (item 1, believed W35-unrelated and
confirmed so), and `table.wast`'s oversized-declared-minimum `module`
case (item 3, left as "unconfirmed as a real bug — needs spec-text
verification before fixing"). No code changes in this crate itself — both
fixes live in `wasm-wast-parser`/`wasm-module-encoder` (gap 1) and
`wasm-validator`/`wasm-runtime`/`wasm-execution` (gap 2); see each crate's
own CHANGELOG for the full root-cause writeups. This entry records the
corpus-wide verification.

**Gap 1** — root-caused via a throwaway probe (outside this repo, not
committed) using `run_wast_source` directly against `elem.wast`, pinning
the failure to directive `#133` (line 1029): a module importing another's
exported externref table and writing to it via an ACTIVE element segment
using the exprs-list form (the only way to express a `(ref.null extern)`
entry) never built at all, because `wasm-wast-parser` rejected that
construct outright — not a table cross-instance-sharing bug (W28's
`Rc<RefCell<TableStorage>>` sharing is correct and was never at fault
here).

**Gap 2** — read `table.wast` line 9 directly: `(module definition (table
0xffff_ffff funcref))` is a bare, UNWRAPPED directive (no `assert_invalid`
around it), meaning the official testsuite itself asserts this
declaration must validate. Confirmed against the real spec's own
implementation-limits reasoning (also already present, if incompletely
followed through, in `wasm-validator`'s own prior Check 2b doc comment):
a 32-bit table's `min` has no spec ceiling below `2^32 - 1`; only actual
allocation may be refused, and only at instantiation time.

**Verification**: regenerated `tests/fixtures/testsuite-status.json` and
diffed all 257 files programmatically (Python, comparing the `files` dict
keyed by filename) against the pre-fix baseline. Exactly 3 files changed:
`elem.wast` (assert_return 18/1-fail/8-not-yet-supported →
24/0-fail/3-not-yet-supported; module 26/50-nys → 36/40-nys; assert_trap
1/6-nys → 4/3-nys; assert_invalid 23/3-nys → 21/5-nys, an HONEST regrade —
two cases that used to "pass" only because the old exprs-list rejection
happened to reject the module for an unrelated reason now correctly
report `NotYetSupported`, since this repo's structural-only validator
genuinely can't tell those specific cases are invalid), `table.wast`
(module 11/1-fail/6-nys → 12/0-fail/6-nys, exactly the one targeted
directive), and `ref_func.wast` (module 2/1-nys → 3/0-nys — the exact same
gap-1 root cause at a different line, `(elem (table $t) (i32.const 0)
funcref (ref.func $f4))`, confirmed via the same probe technique, not an
unrelated regression). Every other of the 257 files is byte-identical to
the pre-fix baseline. Aggregate result: every graded directive category
in the whole corpus is now at 100% (`module` 1982/1983 → 1994/1994,
`assert_return` 51778/51779 → 51784/51784) — the only non-100% categories
left are legitimately-deferred features (`not_yet_supported`), matching
this campaign's own standing discipline of never taking credit for an
untested case.

