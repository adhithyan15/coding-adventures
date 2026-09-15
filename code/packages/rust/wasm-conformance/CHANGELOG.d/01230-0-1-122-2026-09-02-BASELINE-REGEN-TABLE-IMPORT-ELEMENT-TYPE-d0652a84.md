## 0.1.122 — 2026-09-02 — baseline regen: table-import element-type check (W37 addendum)

Regenerated `tests/fixtures/testsuite-status.json` (`--write-baseline`)
after `wasm-execution` 0.9.94 / `wasm-runtime` 0.6.31 fixed the table-
import element-type-mismatch gap 0.1.121's own baseline-regen entry
below reported as a real, un-fixed `Fail` (surfaced by W37's table-
declaration GC-reftype acceptance, PR #14072). No code changes in this
crate itself.

**Diffed programmatically against the pre-fix baseline across all 257
files** (Python, comparing the `files` dict keyed by filename) -- exactly
ONE file changed:

| File | assert_unlinkable before | after |
|---|---|---|
| `linking.wast` | 48 pass / 2 fail | 50 pass / 0 fail |

Both of `linking.wast`'s previously-`Fail` `assert_unlinkable` directives
(importing `$Mtable_ex`'s real funcref-family `t-funcnull`/`t-refnull`
tables as a declared `externref` import) now correctly reject at link
time. Every other file's tally, including every other table-importing
fixture in the corpus, is byte-for-byte unchanged -- confirming the fix is
exactly as narrow as intended.

