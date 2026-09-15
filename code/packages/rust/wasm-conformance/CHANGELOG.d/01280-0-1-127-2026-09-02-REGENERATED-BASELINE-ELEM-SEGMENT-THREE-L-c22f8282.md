## 0.1.127 — 2026-09-02 — regenerated baseline: elem-segment three-layer fix + `array.init_elem`/`array.new_elem` -- W38 slices 4/5 (6 files)

No code changes in this crate — regenerated `tests/fixtures/testsuite-
status.json` (`--write-baseline`) after `wasm-wast-parser`/`wasm-
execution`/`wasm-validator`/`wasm-runtime` implemented the elem-segment
three-layer fix and `array.init_elem`/`array.new_elem`
(`code/specs/W38-wasm-gc-array-bulk-ops.md` slices 4/5), and after two
real, corpus-caught bugs (an active-elem-application fix in `wasm-
runtime`, and an `Element::declared_type` bounds-check gap in `wasm-
validator`) found by re-probing the FULL 257-file corpus were fixed in
the same PR — see those crates' own CHANGELOGs for the full traces.
Diffed programmatically (Python, comparing the `files` dict) against the
pre-slice-4 baseline (0.1.126) across all 257 files — exactly SIX files'
tallies changed, zero elsewhere:

- `array.wast`: module 5/7→6/7, assert_return 17/24→24/24, assert_trap
  12/17→17/17 (14 `not_yet_supported`→1) — the elem-segment reftype/
  const-value gap (13) and `array.new_elem`'s own remainder resolve;
  exactly ONE `not_yet_supported` remains, the pre-existing, out-of-scope
  `(ref struct)` non-null abstract heap type W37 already flagged.
- `array_init_elem.wast`: module 0/3→1/3, assert_return 0/16→7/16, assert_
  trap 0/14→12/14 (33 `not_yet_supported`→13) — 13 remain, ALL tracing to
  `ref.eq` (already flagged out of scope by W37, unrelated to this
  cluster's own instructions — confirmed by direct re-probe, not assumed).
- `array_new_elem.wast`: module 0/5→4/5, assert_return 0/11→10/11, assert_
  trap 0/8→8/8 (24 `not_yet_supported`→2) — the 2 remaining are also
  `ref.eq`.
- `global.wast`: assert_return 61/66→66/66, module 8/9→9/9 (5 `not_yet_
  supported`→0) — pure improvement, an incidental corpus win from the
  same Layer 1 generalization (an active elem segment's own `global.get`
  item now parses AND, after the `wasm-runtime` fix above, executes
  correctly).
- `ref_is_null.wast`: action 0/2→2/2, assert_return 0/16→16/16, module
  0/2→1/2 (18 `not_yet_supported`→0) — same incidental win.
- `elem.wast`: module 50/76→51/76 (net +1 real pass); assert_invalid
  21/26→17/26 (5 `not_yet_supported`→9, net -4 — see below); assert_return
  26/27→26/27 with the ONE `not_yet_supported` becoming a real `fail`
  (see below). **Two distinct, honestly-diagnosed trade-offs, both
  re-verified by directly diffing per-directive outcomes (not inferred
  from aggregate counts), neither hidden**:
  - 4 `assert_invalid` cases (`elem.wast`'s own "Invalid elements"
    section: a numeric/2-instruction/non-constant item in a `funcref`-
    tagged segment) move from an ACCIDENTAL `Pass` (the item's shape used
    to be a hard PARSE error for the wrong reason) to an honest `not_yet_
    supported` ("no instruction-level type-checker; module structurally
    validates") — this crate has NO constant-expression type/arity
    checker anywhere (confirmed by grep: zero hits for "constant
    expression"/"ConstantExpr" in `wasm-validator`), so once parsing is
    correctly permissive, these 4 join the ALREADY-substantial `not_yet_
    supported` category shared by every OTHER "no instruction-level type-
    checker" case across this crate's own `assert_invalid` grading — not
    a new class of gap, just 4 more instances of an existing, accepted
    one.
  - 1 `assert_return` (`elem.wast`'s own "Initializing a table with
    imported funcref global" test) moves from `not_yet_supported` (failed
    to parse) to a real `fail` (safely trapped "call stack exhausted", not
    a crash) — see `wasm-runtime`'s own CHANGELOG for the full diagnosis:
    a genuine, pre-existing, W35-documented cross-instance-funcref-
    through-an-imported-global limitation, newly EXPOSED (not introduced)
    by this spec's own correct parser generalization. Explicitly flagged
    as a follow-up, not silently absorbed into this slice's own scope.

Zero regressions elsewhere: every one of the other 251 files' tallies is
byte-for-byte identical to the pre-slice-4 baseline (re-verified via a
programmatic Python diff of the `files` dict), `fail`/`trap` counts are
`0` everywhere else both before and after, and `parse_failures` is empty
both before and after.

Net aggregate: `assert_return` 52391/52391 (373 NYS) → 52436/52437 (327
NYS, 1 fail); `assert_invalid` 2659/2659 (103 NYS) → 2655/2655 (107 NYS);
`assert_trap` 4903/4903 (65 NYS) → 4928/4928 (40 NYS); `module` 2152/2152
(101 NYS) → 2161/2161 (92 NYS); `action` 383/383 (32 NYS) → 385/385 (30
NYS) — every other directive kind byte-for-byte unchanged.

