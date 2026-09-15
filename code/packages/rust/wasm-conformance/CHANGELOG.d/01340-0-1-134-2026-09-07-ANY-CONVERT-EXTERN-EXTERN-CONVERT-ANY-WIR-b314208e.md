## 0.1.134 — 2026-09-07 — `any.convert_extern`/`extern.convert_any` wired; baseline refresh (W39 slice 3)

Per `code/specs/W39-wasm-gc-ref-eq-cast-br-on-cast.md`, slice 3 of 5.
`wasm-wast-parser` 0.1.108 / `wasm-validator` 0.2.94 / `wasm-execution`
0.9.100 wire `any.convert_extern`/`extern.convert_any` -- see those
crates' own CHANGELOGs for the full account.

**New: `EXTERN_HANDLE_BASE` -- a real, corpus-proven bug in THIS crate's
own test harness, found (not assumed) while diagnosing five unexpected
`assert_return` FAILs in `ref_test.wast` that appeared only after
`any.convert_extern` started actually flowing values through `ref.test`'s
own dynamic check for the first time.** `(ref.extern n)` script literals
and `wasm-execution`'s own `gc_heap` indices both live in the same
`WasmValue::Ref(Option<u32>)` numeric space with nothing to tell them
apart -- `ref_test.wast`'s own "Abstract Types" module uses `(ref.extern
0)` as `init`'s externref parameter, and that SAME module's first real GC
allocation (`struct.new_default $st`) ALSO lands at `gc_heap[0]`, so
`any.convert_extern`'s output was numerically indistinguishable from a
live struct purely by coincidence. Fixed by reserving bit 30
(`EXTERN_HANDLE_BASE = 0x4000_0000`) for every `ref.extern`-sourced
handle, applied once in `const_value_to_wasm_value`/`value_matches_
expected` (a bijection for any real corpus literal, so identity
comparisons are unaffected). **A first attempt reserved bit 31 instead
and immediately regressed seven unrelated files** (`table_get(64).wast`,
`table_fill(64).wast`, `table_grow(64).wast`, `table_set(64).wast`,
`global.wast`, `elem.wast`, `ref_is_null.wast`) because `wasm-execution`'s
own `FUNC_REF_HANDLE_TAG` already reserves that exact bit to mark a
tagged `func_ref_heap` handle -- caught live by the mandatory full
257-file diff, not assumed; see `wasm-execution`'s own CHANGELOG entry
for that constant.

**Corpus delta**, `--write-baseline` diffed programmatically (Python,
comparing the `files` dict) against the pre-slice-3 baseline across all
257 files -- exactly ONE file changed, `ref_test.wast`, from 68
`not_yet_supported` to 0 (68 net movements: 66 `assert_return` + 1
`action` + 1 `module`, all becoming `pass`), with **zero new `fail`/`trap`
anywhere in the full 257-file corpus**:

| File | Before (pass/fail/trap/NYS, summed across all directive kinds) | After |
|---|---|---|
| `ref_test.wast` | 3/0/0/68 (module 1/0/0/1, action 0/0/0/1, assert_return 2/0/0/66) | 71/0/0/0 (module 2/0/0/0, action 1/0/0/0, assert_return 68/0/0/0) |

**`ref_cast.wast` does NOT close in this slice -- an honest gap, not
this slice's own remit.** The spec's own "Recommended slice decomposition"
projected slice 3 would close the remainder of both `ref_test.wast` AND
`ref_cast.wast`'s "Abstract Types" modules. Direct re-verification (a
throwaway probe, same method this spec's own prior slices used) found
`ref_test.wast`'s own module parses and passes completely once `any.
convert_extern`/`extern.convert_any` are wired, but `ref_cast.wast`'s
"Abstract Types" module is blocked by an entirely SEPARATE, unimplemented
instruction: `ref.as_non_null` (confirmed zero references anywhere in
`wasm-wast-parser`/`wasm-execution`/`wasm-validator` -- not merely
unwired, never even started). This instruction has its own opcode, own
runtime semantics, and no overlap with `ref.eq`/`ref.test`/`ref.cast`/
`any.convert_extern`/`extern.convert_any` at all -- the same class of
"real but unrelated" blocker this spec's own "Correction 1" already
established a precedent for (`i31.wast`'s `table.size`/`table.grow`
flat-form gap), so implementing it here would be scope creep beyond this
slice's own five-instruction remit, not a natural extension of it. Left
for a future slice; `ref_cast.wast` remains at its pre-slice-3 3/0/0/42
(pass/fail/trap/NYS) totals, completely unchanged (0 files outside
`ref_test.wast` changed at all, confirmed by the same full-corpus diff).

`wasm-wast-parser` 0.1.107 / `wasm-validator` 0.2.93 / `wasm-execution`
0.9.99 / `wasm-runtime` 0.6.37 extend `ref.test`/`ref.cast` past
concrete-function-only support (per `code/specs/
W39-wasm-gc-ref-eq-cast-br-on-cast.md`, slice 2 of 5) -- see those crates'
own CHANGELOGs for the full account. This crate's own code is unchanged;
only `tests/fixtures/testsuite-status.json` moves, regenerated via
`--write-baseline` and diffed programmatically (Python, comparing the
`files` dict) against the prior baseline across all 257 files. Exactly 3
files changed, all strictly `not_yet_supported` -> `pass` movement, with
**zero new `fail`/`trap` anywhere in the full 257-file corpus**:

| File | Before (pass/fail/trap/NYS, totals) | After |
|---|---|---|
| `i31.wast` | 27/0/0/46 | 31/0/0/42 |
| `ref_test.wast` | 0/0/0/71 | 3/0/0/68 |
| `ref_cast.wast` | 0/0/0/45 | 3/0/0/42 |

Full aggregate: 64846/0/0/621 -> 64856/0/0/611 (-10 `not_yet_supported`,
exactly 4+3+3, zero new failures/traps anywhere in the 257-file corpus).

**Honest scope note, not the spec's original aspiration**: the spec's own
"Recommended slice decomposition" expected this slice to close "most of
`ref_test.wast` (71) and `ref_cast.wast` (45)." Direct re-verification
against the actual pinned corpus found something narrower: BOTH files
split into two independent modules, an "Abstract Types" module (the bulk
of each file's directives) and a "Concrete Types" module (a handful of
directives testing struct nominal subtyping). The "Abstract Types"
module's own `init` function calls `any.convert_extern`/
`extern.convert_any` -- neither instruction is wired into `wasm-wast-
parser`'s text parser at all yet (confirmed: `grep` finds zero hits
outside `wasm-module-encoder`'s own doc comments), so that ENTIRE module
fails to parse, blocking every directive that depends on it regardless of
how complete this slice's own `ref.test`/`ref.cast` work is. Only the
"Concrete Types" module in each file -- unaffected by that dependency --
actually closes here. The remaining 68/42 `not_yet_supported` directives
in `ref_test.wast`/`ref_cast.wast` are confirmed to be exactly this
slice-3 (`any.convert_extern`/`extern.convert_any`) dependency, not a gap
in this slice's own struct/array/abstract-heap-type work.

`i31.wast`'s own 4-directive close matches the spec's Correction 1
prediction exactly (a bare `(ref.cast i31ref ...)` atom) -- its remaining
42 `not_yet_supported` are the two explicitly-out-of-scope causes the spec
itself already named (a flat-form `table.size $table` parsing gap, and an
inline table-initializer expression gap), unrelated to this spec.

An EARLIER attempt at this slice widened `wasm-wast-parser`'s shared
`parse_value_type` (rather than handling `ref.test`/`ref.cast`'s own new
heap-type forms locally) and was caught, by this exact programmatic diff,
regressing `ref_eq.wast` (3 `assert_invalid` directives silently flipped
from correctly-rejected to wrongly-accepted) -- reverted before this
baseline was written; see `wasm-wast-parser`'s own CHANGELOG for the full
story. This is exactly the discipline this crate's own diff methodology
exists to catch.

