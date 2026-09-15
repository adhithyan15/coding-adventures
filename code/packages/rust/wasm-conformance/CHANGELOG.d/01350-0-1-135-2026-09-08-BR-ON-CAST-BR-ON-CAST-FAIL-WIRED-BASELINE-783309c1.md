## 0.1.135 — 2026-09-08 — `br_on_cast`/`br_on_cast_fail` wired; baseline refresh (W39 slice 4)

Per `code/specs/W39-wasm-gc-ref-eq-cast-br-on-cast.md`, slice 4 of 5.
`wasm-wast-parser` 0.1.109 / `wasm-validator` 0.2.95 / `wasm-execution`
0.9.101 / `wasm-runtime` 0.6.38 wire `br_on_cast`/`br_on_cast_fail` --
see those crates' own CHANGELOGs for the full account, including two
genuinely pre-existing gaps (a `decode_blocktype` byte-range hole and a
missing `is_assignable` subtyping edge) found live by this slice's own
corpus re-verification and fixed in the same pass.

**Corpus delta**, `--write-baseline` diffed programmatically (Python,
comparing the `files` dict) against the pre-slice-4 baseline across all
257 files -- exactly THREE files changed, with **zero new `fail`
anywhere in the full 257-file corpus**:

| File | Before | After | Net |
|---|---|---|---|
| `br_on_cast.wast` | module 0/0/0/3, action 0/0/0/3, assert_invalid 6/0/0/0 (31 NYS total) | module 2/0/0/1, action 2/0/0/1, assert_invalid 0/0/0/6 (33 NYS total) | +2 NYS |
| `br_on_cast_fail.wast` | module 0/0/0/3, action 0/0/0/3, assert_invalid 6/0/0/0 (31 NYS total) | module 2/0/0/1, action 2/0/0/1, assert_invalid 1/0/0/5 (32 NYS total) | +1 NYS |
| `type-subtyping.wast` | module 37/0/0/9 (13 NYS total) | module 38/0/0/8 (12 NYS total) | -1 NYS |

(tuples are pass/fail/trap/not_yet_supported)

**Honest accounting -- this is NOT the "31 -> ~0" outcome the spec's own
"Recommended slice decomposition" projected, and the two target files'
own NYS totals went UP, not down. Both effects are real and expected,
not a regression (`fail` stayed at 0 everywhere), for two distinct,
investigated-not-assumed reasons:**

1. **The dominant blocker, discovered live: `br_on_null`/`br_on_non_null`
   are ALSO completely unimplemented in this codebase** (confirmed by a
   `grep` across every crate -- zero hits for either name outside the
   corpus fixtures themselves). Both vendored files open with an
   "Abstract Types" module whose real, rich `br_on_i31`/`br_on_struct`/
   `br_on_array`/`null-diff` test functions all lead with a `br_on_null`/
   `br_on_non_null` call -- a genuinely SEPARATE instruction pair from a
   different proposal, entirely out of this slice's own W39 scope (the
   spec's own opcode table lists only `ref.eq`/`ref.test`/`ref.cast`/
   `br_on_cast`/`br_on_cast_fail`/`any.convert_extern`/`extern.convert_
   any`). This single missing pair gates 27 of each file's own 31
   pre-slice NYS directives -- the exact same "flag it, don't implement
   it" pattern slice 3's own agent already set for `ref_cast.wast`'s
   `ref.as_non_null` gap. Flagged as a follow-up, not implemented here.
2. **The remaining ~6 NYS increase per file is `br_on_cast`/`br_on_cast_
   fail` now genuinely REACHABLE for the first time, exposing that this
   slice's own DELIBERATE scope boundary (no `rt1\rt2`/label-type
   validation, per the spec's own "Explicitly out of scope" item 3) means
   6 (resp. 5) `assert_invalid` cases that used to trivially "pass" ONLY
   because the whole file failed to parse for an unrelated reason now
   correctly show `NotYetSupported` instead -- a MORE honest outcome, not
   a worse one: those directives were never actually exercising real
   rejection logic before.

**Real, positive, corpus-grounded progress this slice DOES prove**: both
files' "Concrete Types"/setup modules (previously 100% blocked) now
parse, structurally validate, and EXECUTE for real -- `br_on_cast.wast`'s
own `test-sub`/`test-canon` functions exercise `br_on_cast` against an
eight-struct-type nominal subtyping lattice (`$t0`../`$t4`, canonical
equivalence via `$t0'`/`$t1'`/`$t2'`) end-to-end and pass. The
null-handling directionality the spec's own "Verification plan" calls
out as the one easy-to-get-backwards piece of runtime logic is verified
by 4 direct unit tests in `wasm-execution` (all four `ht2`-nullable ×
instruction combinations), since the corpus itself cannot currently
reach that code path end-to-end (blocked by `br_on_null` above).

