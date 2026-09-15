## 0.1.96 — 2026-08-26 — W27: census-driven batch, 70 files vendored

### Context

A census found 146/257 upstream `.wast` files vendored at the pinned
commit, 112 missing. Most of the missing 112 fell into three buckets —
files genuinely blocked on this crate's lack of a non-null concrete ref
type (`(ref $t)`), table64 REAL operations (deferred scope from W26),
and plain corpus splits of already-fully-implemented MVP features that
had simply never been fetched. This pass worked through the full list,
live — fetching each candidate from the pinned SHA and actually running
it through this crate's own harness, not trusting any prior
categorization.

### Added — 70 files vendored to `TESTSUITE_FILES`

`address0.wast`, `address1.wast`, `align0.wast`, `binary0.wast`,
`data0.wast`, `data1.wast`, `data_drop0.wast`, `exports.wast`,
`exports0.wast`, `float_exprs0.wast`, `float_exprs1.wast`,
`float_memory0.wast`, `imports0.wast`, `imports1.wast`, `imports2.wast`,
`imports3.wast`, `imports4.wast`, `linking2.wast`, `load0.wast`,
`load2.wast`, `local_init.wast`, `memory_copy0.wast`,
`memory_copy1.wast`, `memory_fill0.wast`, `memory_init0.wast`,
`memory_size0.wast`, `memory_size1.wast`, `memory_size2.wast`,
`memory_size3.wast`, `memory_size_import.wast`, `memory_trap0.wast`,
`memory_trap1.wast`, `names.wast`, `ref_func.wast`, `ref_is_null.wast`,
`skip-stack-guard-page.wast`, `start.wast`, `start0.wast`,
`store0.wast`, `store1.wast`, `store2.wast`, `table-sub.wast`,
`table_grow.wast`, `token.wast`, `traps0.wast`, `unwind.wast`,
`utf8-custom-section-id.wast`, `utf8-import-field.wast`,
`utf8-import-module.wast`, `simd_linking.wast`, `simd_memory-multi.wast`,
`i32x4_relaxed_trunc.wast`, and — a genuine pleasant surprise — 18 GC-
proposal files that turned out to gracefully degrade to all-`not_yet_
supported`/zero-real-`fail` rather than hard-blocking on the missing
concrete-ref-type wall: `ref.wast`, `br_on_null.wast`, `array_copy.wast`,
`array_fill.wast`, `array_init_data.wast`, `array_init_elem.wast`,
`br_on_cast.wast`, `br_on_cast_fail.wast`, `br_on_non_null.wast`,
`call_ref.wast`, `ref_as_non_null.wast`, `ref_cast.wast`, `ref_eq.wast`,
`ref_test.wast`, `return_call_ref.wast`, `type-canon.wast`,
`type-equivalence.wast`, `binary-gc.wast`.

Real, measured aggregate delta (`testsuite-status.json`, full before/
after diff — confirmed byte-for-byte unchanged on every one of the
146 previously-vendored files except one incidental fix, see below):
216 files parsed (0 failed to parse), up from 146; aggregate graded
pass rate 99.958% (52863/52885), up from 99.955% (50782/50805) on the
smaller corpus; total real `fail` count across the ENTIRE corpus went
DOWN by one (21 → 20) even as 70 files' worth of new directives were
added, because of the `linking.wast` fix below. Zero new `fail`
introduced anywhere.

### Fixed (in dependency crates, exercised here)

Three small, real, previously-undetected gaps, each confirmed by
actually running the affected files before AND after the fix:

1. **Active data segments could only ever target memory 0** in a multi-
   memory module — `wasm-validator` 0.2.71 widened Check 8 from "must be
   0" to a real bounds check against the actual memory count;
   `wasm-runtime` 0.6.12 widened `instantiate()` to apply each segment to
   its own `seg.memory_index`. The same pass also fixed a second, real
   bug the widening surfaced: a PASSIVE segment was wrongly required to
   target a real memory even though it references none at all
   (`token.wast`'s own no-memory module). Unblocks `address0.wast`/
   `address1.wast`/`binary0.wast`/`data_drop0.wast`/`float_exprs1.wast`/
   `float_memory0.wast`/`imports2.wast`/`linking2.wast`/`load0.wast`/
   `memory_trap1.wast`/`start0.wast`/`store2.wast`/`token.wast`.
2. **`(kind $name (export "e") (import "m" "n") ...)`** — an inline
   export combined with an inline import on the same field — wasn't
   desugared at all. `wasm-wast-parser` 0.1.82 fixed
   `desugar_one_inline_import` to recognize this shape. Unblocks
   `imports4.wast`/`table_grow.wast`.
3. **A module's `start` function was parsed but never invoked.**
   `wasm-runtime` 0.6.12's `instantiate()` now calls it, as the last
   step, exactly once, if present. Unblocks `start.wast`/`start0.wast`,
   and incidentally fixed one of `linking.wast`'s (already vendored)
   pre-existing `assert_unlinkable` fails (49/50 → 50/50 — confirmed via
   the full baseline diff this is the ONLY tally change among all 146
   previously-vendored files).

`wasm-wast-parser` 0.1.82 and `wasm-execution` 0.9.72 also gained wider
`ref.null` heap-type-keyword support (`any`/`exn`/`none`/`nofunc`/
`noextern`/`noexn`, plus `evaluate_const_expr`'s missing `0xD0` case for
a global's init expression) while investigating `ref_null.wast` — real,
generically-useful fixes, though that specific file stays unvendored
(see below).

### Deliberately NOT vendored (with reasons)

- **`binary.wast`, `binary-leb128.wast`, `binary_leb128_64.wast`** — real,
  pre-existing gaps in malformed-binary-encoding detection (bad LEB128
  widths/overflow bits, section ordering, etc.). `assert_malformed`'s
  BINARY variant (unlike its `quote`/text sibling) has no `not_yet_
  supported` escape hatch in this crate's grading — any case that
  parses+validates when it shouldn't is a hard `fail`, not a capability
  gap. `binary_leb128_64.wast` traced to one specific root cause (a
  canonical-LEB128-encoding check missing for a `memarg` offset,
  compounded by `wasm-execution`'s own `decode_leb_u64`/`decode_leb_u32`
  silently swallowing a decode error into `(0, 1)` instead of surfacing
  it) — real, but nontrivial enough (would need eager per-instruction
  decoding at module-parse time, not just at first-call time) to leave
  for its own future pass rather than folding into this one.
- **`data.wast`** — the SEPARATE extended-const proposal (`i32.add`/
  `i32.sub` inside a data segment's offset expression). This crate's
  `evaluate_const_expr` is a single-accumulator evaluator, not a real
  operator stack; supporting binary operators needs an actual refactor,
  not a small addition.
- **`elem.wast`, `instance.wast`, `linking0.wast`, `linking1.wast`,
  `linking3.wast`, `load1.wast`** — a cross-instance imported memory or
  table is a CLONE in this crate, not a live shared view
  (`RegistryHost::resolve_memory`/`resolve_table`'s own, already-
  documented, deliberate limitation) — a write through one instance's
  imported memory/table isn't observable via a sibling instance that
  also imports it. `instance.wast` additionally needs the separate
  `(module definition ...)`/`(module instance ...)` generative-
  instantiation directive form, which this crate has no support for at
  all (distinct from ordinary `register`).
- **`call_indirect64.wast`, `table_copy64.wast`, `table_copy_mixed.wast`,
  `table_fill64.wast`, `table_get64.wast`, `table_grow64.wast`,
  `table_init64.wast`, `table_set64.wast`, `table_size64.wast`** — table64
  REAL operations (table.get/set/grow/size/fill/copy/init/call_indirect
  against an `is64` table). Confirmed, by actually running each file,
  still exactly the scope boundary W26's own spec deliberately drew —
  real `TypeMismatch` validation fails in every case, not a graceful
  degrade.
- **`array.wast`, `array_new_data.wast`, `array_new_elem.wast`,
  `struct.wast`** — fail to PARSE entirely (not just degrade
  gracefully): real array/struct type-declaration grammar this crate's
  `wasm-wast-parser` has zero support for.
- **`ref_null.wast`** — mostly fixed (see above), but its final module
  needs a null BOTTOM reference type (`nullfuncref`) to type-check as a
  subtype of a concrete `(ref null $t)` function result type — the same
  non-null-concrete-ref subtyping wall the GC files hit, just reached
  from the opposite (null) direction.
- **`type-rec.wast`, `type-subtyping.wast`** — real recursive-type-group
  and explicit-subtype-declaration semantics; each has 1-2 real `fail`s
  once the rest of the file gracefully degrades, not the clean zero-fail
  bar every other GC file vendored in this batch meets.
- **`annotations.wast`, `inline-module.wast`** — two more genuinely
  separate, unimplemented text-format features: custom `@id` annotation
  syntax (a distinct tokenizer-level feature), and a `.wast` script with
  no enclosing `(module ...)` wrapper at all (a different top-level
  script grammar this crate's `parse_script` doesn't recognize).

See `fetch_testsuite.py`'s own batch-level comment for the same
accounting inline with the vendored file list, and `tests/fixtures/
testsuite/NOTICE` for the provenance entry.

