## 0.1.121 — 2026-09-02 — baseline regen: GC reftype tables (W37)

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-types` 0.1.25 / `wasm-wast-parser` 0.1.100 / `wasm-validator`
0.2.88 / `wasm-execution` 0.9.93 / `wasm-runtime` 0.6.30 landed
`code/specs/W37-wasm-gc-reftype-tables.md`'s table-declaration
reftype-acceptance fix (including a security-review-caught fix to
`call_indirect`/`return_call_indirect`'s own missing funcref-table check
-- see `wasm-validator` 0.2.88's own CHANGELOG). No code changes in this
crate itself.

**Diffed programmatically against the pre-fix baseline across all 257
files** (Python, comparing the `files` dict keyed by filename) -- 6 files
changed:

| File | Pass/Fail/Trap/NYS before | after |
|---|---|---|
| `table-sub.wast` | 0/0/0/3 | 1/0/0/2 |
| `call_indirect.wast` | 160/0/0/12 | 161/0/0/11 |
| `return_call_indirect.wast` | 17/0/0/64 | 18/0/0/63 |
| `ref.wast` | 10/0/0/3 | 9/0/0/4 |
| `table.wast` | 35/0/0/11 | 34/0/0/12 |
| `linking.wast` | 155/0/0/8 | 155/2/0/6 |

**Matches the spec's own prediction for exactly one file**:
`table-sub.wast`'s table-decl-attributable NYS closes (0 -> 1 real Pass),
as predicted. **Two additional, unplanned real improvements** came from
the security-review-caught `call_indirect`/`return_call_indirect` fix (not
part of the spec's own design, found during this PR's mandatory security
review round): `call_indirect.wast`'s and `return_call_indirect.wast`'s
own "expects funcref type but receives externref" `assert_invalid` cases,
previously misclassified `NotYetSupported` ("structurally validates")
since nothing checked a table's element type at all for these two
opcodes, now correctly `Fail` the module as the real spec requires.

**Diverges from the spec's own prediction for `ref_cast.wast`**: the spec
predicted this file (45 NYS) was "the most likely to reach real Pass."
Live re-probe (`run_wast_source`, the same throwaway-probe method the spec
itself used) found its total NYS count is UNCHANGED (45 before and after)
-- every one of its table declarations now parses, but the file's two
modules each hit a SEPARATE, real, pre-existing blocker the spec's own
per-file prediction did not anticipate: module 1 ("Abstract Types") calls
`any.convert_extern` (confirmed entirely unimplemented, the same
instruction the spec itself already flagged as a `ref_test.wast` blocker,
just not cross-checked against `ref_cast.wast`'s own source); module 2
("Concrete Types") casts to a concrete STRUCT type via `ref.cast`, which
hits `wasm-wast-parser`'s own pre-existing restriction to concrete FUNC-
family type immediates only (`module.rs:4397-4406` -- the spec's own
"Explicitly out of scope" section already flags this restriction as stale,
but attributed it to i31.wast's ABSTRACT-heap-type case, not to
ref_cast.wast's CONCRETE-struct-type case, which trips the identical
restriction for a different reason).

`ref_eq.wast`/`ref_test.wast`/`i31.wast`/`br_on_cast.wast`/`br_on_cast_
fail.wast` all show IDENTICAL pass/fail/NYS totals pre- and post-fix,
matching the spec's own prediction that these would "parse further but
reveal genuinely separate, already-identified" blockers rather than close.
`i31.wast` in particular: live re-probe shows the VAST MAJORITY of its
directives (all `$..._table_of_i31ref`-shaped modules) now fail with
`"expected an index, found \"i31ref\""` -- exactly the spec's own
"Correction 2" elem-segment bare-reftype-keyword gap (`i31ref` in an
`(elem ...)` reftype position), already flagged there as real but
out-of-scope. The spec's own rough "~38 of 46 resolving" estimate for this
file does not hold once this dominant elem-segment gap is accounted for --
0 of its 46 close.

`ref.wast`/`table.wast` show a small number of `Pass` -> `NotYetSupported`
shifts (never `Fail`) -- see `wasm-wast-parser` 0.1.100's own CHANGELOG for
the root cause (pre-existing "no instruction-level type-checker"/non-null-
abstract-heap-type gaps becoming reachable in a table-declaration position
for the first time, not a new class of gap).

`linking.wast` shows 2 genuine new `Fail`s -- see `wasm-runtime` 0.6.30's
own CHANGELOG: a pre-existing, already-self-documented table-import
element-type-checking gap in `wasm-runtime::instantiate`, newly reachable
because the one corpus fixture exercising it needed this exact release to
parse at all. Flagged as a follow-up, not fixed here.

**Bottom line**: of the cluster's 550 NYS this spec's own motivating
document cited, the REALIZED improvement from this release is 1 directive
(`table-sub.wast`) closing to real `Pass` -- smaller than the spec's own
"~45-46" prediction, entirely explained by two things the spec's own
per-file research did not fully cross-check: `ref_cast.wast`'s `any.
convert_extern`/concrete-struct-`ref.cast`-restriction blockers, and
`i31.wast`'s dominant elem-segment gap. The fix itself is correct and
matches the real GC/function-references proposal grammar exactly; the
corpus impact is simply smaller than hoped because other, separate,
already-identified gaps dominate the downstream path for every other file
in the cluster.

