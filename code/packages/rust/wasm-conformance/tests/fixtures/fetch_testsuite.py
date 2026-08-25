#!/usr/bin/env python3

"""Vendor a pinned slice of the official WebAssembly/testsuite corpus.

`wasm-conformance` (added in a later PR; see
`code/specs/W05-wasm-conformance-harness.md`) runs the real
`WebAssembly/testsuite` `.wast` scripts against this repo's `wasm-execution`
interpreter and `wasm-wast-parser` text-format parser. That corpus is fetched
from a **pinned commit SHA**, never `main` -- `main` already interleaves
GC/exceptions/tail-call/SIMD proposal files with plain MVP-core ones, so an
unpinned fetch would produce a different, non-reproducible baseline on every
re-run.

This script re-downloads the exact same file list this repo vendors under
`testsuite/`, verbatim, from that one pinned SHA -- so re-running it is a
no-op unless `PINNED_SHA` below is bumped on purpose (a deliberate, reviewed
decision, same as any other dependency version bump).

Usage:
    python3 fetch_testsuite.py
"""

from __future__ import annotations

import urllib.request
from pathlib import Path

REPO = "WebAssembly/testsuite"

# Pinned via `git ls-remote https://github.com/WebAssembly/testsuite.git
# refs/heads/main` on 2026-08-13. Bump deliberately, not casually -- see the
# module docstring.
PINNED_SHA = "28864811cf03bdbf880733786148feaba339582d"

FIXTURES_DIR = Path(__file__).parent
OUTPUT_DIR = FIXTURES_DIR / "testsuite"

# WASM 1.0 MVP core only (extended once, WASM08, beyond the initial
# W05/PR3 slice -- see this file's own git history for the two-stage
# growth). Deliberately excludes anything needing the `spectest`
# host-import module or heavier module-linking semantics, and anything
# from a post-MVP proposal (SIMD, threads/atomics, exceptions, tail
# calls, bulk memory operations, GC/reference-types beyond this repo's
# existing narrow slice, memory64, the component model) -- see
# `code/specs/W05-wasm-conformance-harness.md` section 6 for the original
# out-of-scope list and why each is deferred to its own future phase, not
# vendored here.
TESTSUITE_FILES = [
    # Numerics/literals
    "i32.wast",
    "i64.wast",
    "f32.wast",
    "f64.wast",
    "f32_bitwise.wast",
    "f64_bitwise.wast",
    "f32_cmp.wast",
    "f64_cmp.wast",
    "int_exprs.wast",
    "int_literals.wast",
    "float_literals.wast",
    "float_exprs.wast",
    "float_misc.wast",
    "conversions.wast",
    "const.wast",
    "left-to-right.wast",
    # Control flow
    "block.wast",
    "loop.wast",
    "if.wast",
    "br.wast",
    "br_if.wast",
    "br_table.wast",
    "return.wast",
    "labels.wast",
    "nop.wast",
    "unreachable.wast",
    "switch.wast",
    "forward.wast",
    # Type checking / dead code (WASM06's instruction-level type checker)
    "unreached-invalid.wast",
    "unreached-valid.wast",
    # Types
    "type.wast",
    # Calls
    "call.wast",
    "call_indirect.wast",
    "func.wast",
    "func_ptrs.wast",
    "fac.wast",
    "stack.wast",
    # Variables
    "local_get.wast",
    "local_set.wast",
    "local_tee.wast",
    "global.wast",
    # Tables (task #96) -- WASM17's table.get/table.set already existed,
    # but vendoring these surfaced (and this PR fixes) a real bug: table
    # declarations never actually read their `funcref`/`externref` reftype
    # keyword, silently defaulting every table's element_type to funcref
    # regardless of the source (`(table $t2 2 externref)` still parsed as
    # funcref).
    "table_get.wast",
    "table_set.wast",
    # table.wast (task #99): needed hex-literal table/memory limit parsing
    # (`(table 0xffff_ffff funcref)`) -- three independent call sites in
    # wasm-wast-parser filtered atoms to ascii-digit-only, silently
    # dropping a hex atom out of the limits list entirely (or, for the
    # declared-table/imported-table sites, taking the WRONG parse branch
    # altogether) instead of parsing it. One `(table (import "spectest"
    # "table") ...)` directive grades NotYetSupported via the harness's
    # existing unresolved-import handling -- doesn't block the rest of
    # the file.
    "table.wast",
    # table.size/table.grow/table.fill (task #98) -- entirely unimplemented
    # before this pass (no opcode decoding, no interpreter handler, no
    # wast-parser text-form support, no validator type-check rule).
    # Deliberately excludes table_grow.wast: its own corpus uses `(elem
    # declare func $f)` -- declarative element segments, a third element-
    # segment mode this crate has no concept of yet (only active/passive
    # data segments exist, task #95's `is_passive`, and now also passive-
    # exprs-list for elements, task #97's own widening) -- and cross-module
    # `register`/import table-growth propagation tests that are a fine
    # follow-on once `elem declare` lands. table_size.wast and
    # table_fill.wast use neither and are fully vendorable now.
    "table_size.wast",
    "table_fill.wast",
    # table.init/table.copy/elem.drop (task #97) -- were entirely
    # unimplemented (no opcode decoding, no interpreter handler, no
    # wast-parser text-form support, no validator type-check rule, and
    # `Element` had no passive/exprs-list representation at all) before
    # this pass. See `code/specs/W17-wasm-bulk-table-ops.md` for the real-
    # corpus census that scoped the binary encoding to exactly 4 of the 8
    # spec-defined element-segment modes (0/1/2/5) -- these two files use
    # only those. Deliberately excludes their sibling `bulk.wast` (mixes
    # in memory.copy/memory.fill/memory.init/data.drop too, already
    # covered individually above) and `elem.wast` (uses `(elem declare
    # ...)`, the same declarative-segment gap table_grow.wast has).
    "table_init.wast",
    "table_copy.wast",
    # Memory
    "memory.wast",
    "address.wast",
    "align.wast",
    "endianness.wast",
    "load.wast",
    "store.wast",
    "memory_size.wast",
    "memory_grow.wast",
    "memory_trap.wast",
    "traps.wast",
    "float_memory.wast",
    "memory_redundancy.wast",
    # Bulk-memory proposal (task #94): memory.copy already existed
    # (E4-dyn's runtime string concat needed it); memory.fill was added
    # alongside this vendoring pass, and this crate's wast-parser never
    # had text-form support for EITHER instruction (both are 0xFC-
    # prefixed, so -- like trunc_sat/atomics/SIMD -- they need their own
    # interception before `wasm_opcodes::get_opcode_by_name`'s lookup,
    # which only this vendoring pass added). Deliberately excludes their
    # sibling `bulk.wast`: it mixes memory.copy/memory.fill with
    # memory.init/data.drop (task #95) and table.init/elem.drop/
    # table.copy (task #97) in the SAME file, all still unimplemented --
    # only vendorable once both of those land too.
    "memory_copy.wast",
    "memory_fill.wast",
    # memory.init/data.drop (task #95) -- were entirely unimplemented
    # (no opcode decoding, no interpreter handler, no wast-parser text-
    # form support, no validator type-check rule) before this pass;
    # needed real new interpreter state (`WasmExecutionContext::
    # data_segments`/`dropped_data_segments`, persistent across calls
    # like `v128_heap`) and real PASSIVE data segment support (`(data $d
    # "bytes")`, no offset expression -- `wasm-module-parser`'s binary
    # decoder only ever handled segment-mode flag 0x00 before this, not
    # the real 3-mode encoding the bulk-memory proposal defines). Only
    # uses single-memory numeric segment indices (no `spectest` import,
    # no table/elem instructions), so it's vendorable standalone --
    # unlike its sibling `bulk.wast` (task #97 still pending) and
    # `memory-multi.wast` (task #92, explicit multi-memory indices).
    "memory_init.wast",
    # Real multi-memory memarg (W18, task #92/#109-112): `i32.load`'s
    # leading-memidx-token form, plus `memory.init`/`memory.fill` with an
    # explicit non-default memory index -- the binary flags-bit `0x40` +
    # memidx encoding, decoded/executed for real across all 23 memarg
    # opcodes (`wasm-execution`), the matching text-form leading token
    # (`wasm-wast-parser`), and the matching validation-time bounds-check
    # (`wasm-validator`) all landed in this same pass. See `code/specs/
    # W18-wasm-multi-memory-memarg.md`.
    "memory-multi.wast",
    # Real cross-module linking (WASM05, task #93) -- the original W05
    # scope note excluded this as needing "heavier module-linking
    # semantics" than existed at the time; `RegistryHost` (real
    # HostInterface link-failure path, WASM05/W10) now provides exactly
    # that. Of its 71 modules, only 2 import from `spectest` (this crate
    # has no spectest host, by design -- see `RegistryHost`'s own doc
    # comment); every other module/directive exercises real, already-
    # supported cross-module function/table/memory/global import/export/
    # register machinery.
    "linking.wast",
    # Parser self-test
    "select.wast",
    "comments.wast",
    "id.wast",
    "custom.wast",
    "obsolete-keywords.wast",
    "utf8-invalid-encoding.wast",
    # SIMD proposal (v128) -- this repo's first-slice implementation
    # (SIMD PR1a/PR1b, code/specs/W13-wasm-simd-v128-first-slice.md)
    # originally covered only 5 opcodes (v128.const/i32x4.splat/add/eq/
    # extract_lane), so only the narrowest real corpus file was vendored:
    # `simd_const.wast` tests v128.const's OWN literal syntax across all
    # 6 shapes (this repo's wast-parser already handles all 6, SIMD
    # PR1b-2/1b-3) and is almost entirely gradeable already; its one
    # instruction beyond this slice (a single `i64x2.add` line) grades
    # NotYetSupported for just that one directive rather than blocking
    # the whole file (W14, code/specs/W14-wasm-conformance-lazy-module-
    # build.md) -- the first real corpus file this repo could vendor from
    # a post-MVP proposal.
    "simd_const.wast",
    # SIMD widening (task #113-117): the i32x4 lane width's own
    # arithmetic (mul/neg/sub, joining the already-implemented add) and
    # full comparison family (ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u,
    # joining eq) newly unblock these two sibling files -- each opcode's
    # exact sub-opcode byte was fetched live from the SIMD proposal's own
    # BinarySIMD.md and cross-checked against the already-implemented
    # i32x4.eq/i32x4.add entries (both matched exactly). `simd_splat.wast`
    # (the third sibling named alongside these two in this comment's own
    # prior revision) still references many more unsupported opcode
    # families (f32x4/f64x2/i16x8/i8x16 arithmetic, shr_s, all_true,
    # swizzle, saturating add/sub, trunc_sat -- not just more i32x4 ops)
    # and stays deferred to a future widening pass.
    "simd_i32x4_arith.wast",
    "simd_i32x4_cmp.wast",
    # SIMD widening (task #118-120): i32x4.abs (UNARY) and the min_s/
    # min_u/max_s/max_u family -- the "second half" of i32x4 arithmetic
    # coverage this repo's upstream corpus itself splits into a separate
    # file, same real-verified-sub-opcode discipline as the pass above.
    "simd_i32x4_arith2.wast",
    # SIMD widening (task #121-124): i32x4.extadd_pairwise_i16x8_s/_u,
    # i32x4.dot_i16x8_s, i32x4.extmul_low/high_i16x8_s/_u -- the first
    # opcodes in this repo whose INPUT lane width (i16x8) differs from
    # their OUTPUT lane width (i32x4). Each sub-opcode byte fetched live
    # from BinarySIMD.md and cross-checked against the already-implemented
    # i32x4.eq/i32x4.add entries (both matched exactly), same discipline
    # as every widening pass above. Three separate upstream files, one per
    # opcode family, same as this repo's own opcode-family split.
    "simd_i32x4_extadd_pairwise_i16x8.wast",
    "simd_i32x4_dot_i16x8.wast",
    "simd_i32x4_extmul_i16x8.wast",
    # SIMD widen PR4 (task #125-128): simd_i8x16_arith.wast -- the first
    # `i8x16` lane-width slice (add/sub/neg only, no mul since the spec
    # defines none, no splat/extract_lane since v128.const i8x16 already
    # covers this file's own operand construction and result comparison).
    # Each sub-opcode byte fetched live from BinarySIMD.md and cross-
    # checked against the already-implemented i32x4.add/i32x4.abs
    # entries (both matched exactly), same discipline as every prior
    # addition.
    "simd_i8x16_arith.wast",
    # SIMD widen PR5 (task #129-132): simd_i16x8_arith.wast -- the first
    # opcodes where i16x8 is a PRIMARY lane width (add/sub/mul/neg,
    # produces i16x8 results), not just an INPUT to an i32x4-producing
    # widening op (extadd_pairwise/dot/extmul, already implemented).
    # Unlike i8x16, WASM SIMD DOES define i16x8.mul, so this slice
    # includes all four ops the real upstream file bundles together.
    # Each sub-opcode byte fetched live from BinarySIMD.md and cross-
    # checked against the already-implemented i32x4.add/i8x16.add
    # entries (both matched exactly), same discipline as every prior
    # addition.
    "simd_i16x8_arith.wast",
    # SIMD widen PR6 (task #133-136): simd_i16x8_cmp.wast -- i16x8's own
    # comparison family (eq/ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u),
    # closing the gap left when i16x8.add/sub/mul/neg landed without one
    # (unlike i32x4, which got arith+cmp together). Same boolean-mask
    # convention and signed/unsigned split as i32x4's own comparison
    # family, just at the narrower lane width. Each sub-opcode byte
    # fetched live from BinarySIMD.md and cross-checked against the
    # already-implemented i16x8.add/i32x4.eq entries (both matched
    # exactly), same discipline as every prior addition.
    "simd_i16x8_cmp.wast",
    # SIMD widen PR7 (task #137-140): simd_i8x16_cmp.wast -- i8x16's own
    # comparison family (eq/ne/lt_s/lt_u/gt_s/gt_u/le_s/le_u/ge_s/ge_u),
    # closing the same gap PR6 closed for i16x8: i8x16.add/sub/neg landed
    # (PR4) without a comparison family. Same boolean-mask convention and
    # signed/unsigned split as i16x8's and i32x4's own comparison
    # families, just at the narrowest lane width. Each sub-opcode byte
    # fetched live from BinarySIMD.md and cross-checked against the
    # already-implemented i8x16.add/i16x8.eq entries (both matched
    # exactly), same discipline as every prior addition.
    "simd_i8x16_cmp.wast",
    # SIMD widen PR8 (task #141-143): simd_i8x16_arith2.wast -- i8x16's
    # own abs/popcnt/min_s/min_u/max_s/max_u/avgr_u family, mirroring
    # i32x4's own abs/min_s/min_u/max_s/max_u widening (PR2), plus two op
    # SHAPES with no i32x4/i16x8 precedent in this interpreter: popcnt
    # (lane-wise Hamming weight) and avgr_u (lane-wise unsigned rounding
    # average, (a+b+1)>>1) -- WASM SIMD only defines popcnt/avgr_u for
    # i8x16 (avgr_u is also defined for i16x8, but not i32x4). Each
    # sub-opcode byte fetched live from BinarySIMD.md and cross-checked
    # against the already-implemented i8x16.add/i8x16.neg/i8x16.sub
    # entries (all three matched exactly), same discipline as every
    # prior addition.
    "simd_i8x16_arith2.wast",
    # SIMD widen PR9 (task #144-146): simd_i16x8_arith2.wast -- i16x8's
    # own abs/min_s/min_u/max_s/max_u/avgr_u family, closing the same
    # "arith2" gap PR8 just closed for i8x16 (no i16x8.popcnt -- WASM
    # SIMD only defines popcnt for i8x16). Each sub-opcode byte fetched
    # live from BinarySIMD.md and cross-checked against the
    # already-implemented i16x8.neg/add/sub/mul entries (all four
    # matched exactly), same discipline as every prior addition.
    "simd_i16x8_arith2.wast",
    # SIMD widen PR10 (task #147-149): simd_i16x8_extadd_pairwise_i8x16.
    # wast/simd_i16x8_extmul_i8x16.wast -- i16x8-from-i8x16 widening
    # family (extadd_pairwise_i8x16_s/u, extmul_low/high_i8x16_s/u),
    # mirroring the already-implemented i32x4-from-i16x8 widening
    # family one lane width down. No i16x8.dot_i8x16_s -- WASM SIMD
    # does not define a dot-product for this pair. Each sub-opcode
    # byte fetched live from BinarySIMD.md and cross-checked against
    # the already-implemented i8x16.add/i16x8.mul/i16x8.avgr_u/
    # i32x4.dot_i16x8_s/i8x16.popcnt/i32x4.extadd_pairwise_i16x8_s
    # entries (all six matched exactly), same discipline as every
    # prior addition.
    "simd_i16x8_extadd_pairwise_i8x16.wast",
    "simd_i16x8_extmul_i8x16.wast",
    # SIMD widen PR22 (task #183-185): simd_i16x8_q15mulr_sat_s.wast --
    # i16x8.q15mulr_sat_s (0x82), a Q15 fixed-point ROUNDING SATURATING
    # multiply -- the first genuinely new SIMD op family/semantic since
    # the "extmul" widening-multiply arc completed in PR21 (not a plain
    # wrapping/compare/min-max op like every other i16x8 binary entry).
    # Sub-opcode fetched live from BinarySIMD.md and cross-checked
    # against the already-implemented i16x8.neg (0x81)/i16x8.all_true
    # (0x83) entries that straddle it (0x82 was the one gap between
    # them). Same as PR21's file, this is a BRAND-NEW file this repo
    # did not previously vendor at all.
    "simd_i16x8_q15mulr_sat_s.wast",
    # SIMD widen PR11 (task #150-152): simd_bitwise.wast -- v128.not/
    # and/andnot/or/xor/bitselect, the lane-width-agnostic raw-byte
    # bitwise family. A strategic pivot from "widen the next narrow
    # per-lane-width family" (PR1-PR10's pattern) to "close the
    # highest-real-world-impact remaining gap", identified via a
    # broader prioritization survey now that i8x16/i16x8/i32x4 all
    # have complete arith+cmp+arith2+widening coverage. bitselect is
    # the first TERNARY SIMD op in this interpreter. Each sub-opcode
    # byte fetched live from BinarySIMD.md and cross-checked against
    # the already-implemented i8x16.add/i32x4.add entries, same
    # discipline as every prior addition.
    "simd_bitwise.wast",
    # SIMD widen PR12 (task #153-155): simd_boolean.wast -- v128.any_true
    # + ixNxM.all_true/bitmask across all 4 lane widths (i8x16/i16x8/
    # i32x4/i64x2). The first v128-in/i32-out reduction shape besides
    # extract_lane (no lane-index immediate -- reduces over ALL lanes),
    # and the first opcodes in this interpreter to read the operand as
    # 8-byte (i64) lanes. Chosen over the shift-op and i64x2-arithmetic
    # candidates in a broader prioritization survey: highest opcode
    # count (9) behind a single new operand shape and a single 72KB
    # corpus file, and unlocks real use of the comparison families from
    # PR1/PR6/PR7 (a v128 mask result is otherwise inert without a
    # reduction op to consume it). Each sub-opcode byte fetched live
    # from BinarySIMD.md and cross-checked against the already-
    # implemented v128.bitselect/i8x16.popcnt/i16x8.abs/neg/i32x4.abs/
    # neg entries, same discipline as every prior addition.
    "simd_boolean.wast",
    # SIMD widen PR13 (task #156-158): simd_i64x2_arith.wast/
    # simd_i64x2_arith2.wast/simd_i64x2_cmp.wast -- i64x2.abs/neg/add/
    # sub/mul/eq/ne/lt_s/gt_s/le_s/ge_s, i64x2's first REAL ARITHMETIC
    # family (PR12 only added the all_true/bitmask reduction ops). No
    # lt_u/gt_u/le_u/ge_u -- the SIMD proposal never defines unsigned
    # i64x2 comparisons, unlike every narrower lane width. Reuses the
    # existing v128,v128->v128 / v128->v128 shapes already implemented
    # for every other lane width -- this closes a lane-width coverage
    # gap, not a new operand shape. Each sub-opcode byte fetched live
    # from BinarySIMD.md and cross-checked against the already-
    # implemented i64x2.all_true/bitmask entries plus the identical
    # abs/neg/[gap]/all_true/bitmask cluster layout already confirmed
    # for i8x16/i16x8/i32x4.
    "simd_i64x2_arith.wast",
    "simd_i64x2_arith2.wast",
    "simd_i64x2_cmp.wast",
    # SIMD widen PR14 (task #159-161): simd_bit_shift.wast --
    # ixNxM.shl/shr_s/shr_u across all 4 lane widths (i8x16/i16x8/
    # i32x4/i64x2). The FIRST mixed-type binary SIMD op family: pops a
    # scalar i32 shift amount (pushed last, so popped first) then a
    # v128, pushes one v128 -- every prior binary op popped two v128s
    # or one v128, never a mix. Per the SIMD spec, the shift amount is
    # taken MODULO the lane's bit width before shifting (8/16/32/64
    # respectively) -- both spec-mandated and required for Rust safety
    # (shifting a primitive by >= its bit width panics). Each
    # sub-opcode byte fetched live from BinarySIMD.md and cross-checked
    # against the already-implemented per-width `add` entries (every
    # width's shl/shr_s/shr_u triple sits immediately before that
    # width's own `add` sub-opcode).
    "simd_bit_shift.wast",
    # SIMD widen PR15 (task #162-164): simd_load.wast/simd_store.wast --
    # v128.load/v128.store, the FIRST SIMD ops touching real linear memory
    # (every prior SIMD op only reads/writes the per-instance v128 heap).
    # Both single-byte sub-opcodes (0x00/0x0B); scoped to memory-0-only
    # execution for this first PR (see wasm-execution's DecodedOperand
    # packing for why -- multi-memory v128 load/store is deferred).
    # Landing this also retroactively resolves 14 assert_return directives
    # in the already-vendored simd_bitwise.wast (PR11) that were stuck at
    # NotYetSupported pending a real v128.load.
    "simd_load.wast",
    "simd_store.wast",
    # SIMD widen PR16 (task #165-167): simd_splat.wast -- i8x16.splat/
    # i16x8.splat/i64x2.splat (0x0F/0x10/0x12), widening lane-width
    # coverage of the already-implemented i32x4.splat (0x11). Same
    # "pop scalar, push v128" shape; i64x2.splat is the first splat that
    # pops i64 rather than i32. The upstream file bundles all 6 splat
    # exports (including f32x4.splat/f64x2.splat, neither implemented)
    # into ONE module, so that module -- and every directive invoking
    # it -- grades NotYetSupported until float-lane SIMD support lands
    # in a future PR; still vendored now since it's the real upstream
    # file and the 3 new opcodes are correctly implemented and tested
    # via dedicated unit tests in the meantime.
    "simd_splat.wast",
    # SIMD widen PR21 (task #180-182): simd_i64x2_extmul_i32x4.wast --
    # i64x2.extmul_low/high_i32x4_s/_u, the third and final rung of this
    # repo's "extmul" widening-multiply family (i8x16->i16x8, i16x8->
    # i32x4, and now i32x4->i64x2). Mirrors the already-implemented
    # simd_i32x4_extmul_i16x8.wast one lane width up. Each sub-opcode
    # byte fetched live from BinarySIMD.md and cross-checked against the
    # already-implemented i32x4.extmul_low_i16x8_s/i64x2.abs/i64x2.ge_s
    # entries, same discipline as every prior addition. Unlike every
    # other file added to this list so far, this is a BRAND-NEW file
    # this repo did not previously vendor at all -- not a re-fetch of an
    # already-vendored file whose baseline improves via a later PR.
    "simd_i64x2_extmul_i32x4.wast",
    # SIMD widen PR23 (task #186-187): simd_select.wast/simd_address.wast --
    # unlike every prior PR in this campaign, ZERO new opcodes. Both files
    # use only opcodes this interpreter already fully implements:
    # simd_select.wast exercises untyped `select` with v128 operands (the
    # parametric `select` (0x1B) opcode is generic over `WasmValue` in both
    # `wasm-execution` and the validator's type-check rule -- no
    # SIMD-specific special-casing anywhere gates it to scalar types), and
    # simd_address.wast exercises v128.load/v128.store (PR15) memarg
    # offset/align edge cases (including the same `offset=-1` malformed and
    # `offset=4294967296` invalid cases already covered by the vendored
    # load.wast/store.wast/simd_load.wast/simd_store.wast). Verified by
    # actually vendoring and grading both files, not just by static opcode
    # inventory -- both come back 100% passing.
    "simd_select.wast",
    "simd_address.wast",
    # SIMD widen PR24 (task #188-189): simd_i32x4_trunc_sat_f32x4.wast --
    # like PR23, ZERO new opcodes. Exercises only
    # `i32x4.trunc_sat_f32x4_s`/`i32x4.trunc_sat_f32x4_u`, both already
    # implemented since SIMD widen PR20 (task #177-179). Verified by
    # actually vendoring and grading the file, not just a static opcode-
    # inventory claim: 100% pass on every directive (1/1 module, 102/102
    # assert_return, 4/4 assert_invalid).
    "simd_i32x4_trunc_sat_f32x4.wast",
    # SIMD widen PR25 (task #190-192): simd_i32x4_trunc_sat_f64x2.wast --
    # vendors the dedicated upstream file for the 2 NEW opcodes this PR
    # adds: `i32x4.trunc_sat_f64x2_s_zero` (0xFC), `i32x4.trunc_sat_f64x2_
    # u_zero` (0xFD). Mirrors PR24's `simd_i32x4_trunc_sat_f32x4.wast`
    # almost exactly, just the f64x2-source ("_zero") rung instead of the
    # f32x4-source one -- same boundary-value corpus shape (zero/
    # negative-zero/fractional/exact-integer/just-inside-range/
    # just-outside-range/huge-finite/subnormal/inf/nan/signed-and-quiet-
    # nan-payload/octal-literal cases, `_s_zero` and `_u_zero` each tested
    # independently) plus its own `assert_invalid` type-check coverage
    # (wrong-operand-type, empty-argument for both ops).
    "simd_i32x4_trunc_sat_f64x2.wast",
    # SIMD widen PR28 (task #199-201): simd_conversions.wast -- the THIRD
    # and FINAL PR of a 3-PR sequence (PR26 "extend" family, 8 opcodes;
    # PR27 "narrow" family, 4 opcodes; this PR's "promote/demote/
    # convert_low" family, 4 opcodes) needed to land all 16 opcodes this
    # single upstream file's two modules bundle together. Unlike every
    # earlier extend/narrow/widening file above, this file could NOT be
    # partially vendored -- both of its modules export functions that
    # exercise opcodes from ALL THREE of those PRs at once, so PR26 and
    # PR27 each left this file un-vendored (opcode-only, unit-test-
    # verified) until this PR's 4 new opcodes (`f32x4.demote_f64x2_zero`
    # 0x5E, `f64x2.promote_low_f32x4` 0x5F, `f64x2.convert_low_i32x4_s`
    # 0xFE, `f64x2.convert_low_i32x4_u` 0xFF) completed the set. First
    # real integration test exercising opcodes from all three PRs
    # together in one corpus file.
    "simd_conversions.wast",
    # SIMD widen PR29 (task #202-204): simd_f32x4_arith.wast -- vendors
    # the dedicated upstream file for f32x4.neg/sqrt/add/sub/div, this
    # PR's 5 new opcodes (joining the already-implemented abs/mul/min,
    # PR19), closing the last remaining gap in f32x4's core arithmetic
    # family. The single biggest directive-count win in this campaign so
    # far -- see the NOTICE file for the real vendored/pass counts.
    "simd_f32x4_arith.wast",
    # SIMD widen PR30 (task #205-207): simd_f32x4_cmp.wast -- vendors the
    # dedicated upstream file for f32x4.eq/ne/lt/gt/le/ge (0x41-0x46),
    # this PR's 6 new opcodes, closing the f32x4 comparison family gap
    # (the arithmetic family completed in PR29 above). The single BIGGEST
    # directive-count win in this campaign so far -- larger than PR29's
    # simd_f32x4_arith.wast -- see the NOTICE file for the real vendored/
    # pass counts.
    "simd_f32x4_cmp.wast",
    # SIMD widen PR31 (task #208-210): simd_f64x2_arith.wast -- vendors
    # the dedicated upstream file for f64x2.neg/sqrt/add/sub/mul/div
    # (0xED, 0xEF-0xF3), this PR's 6 new opcodes -- a direct structural
    # mirror of PR29's simd_f32x4_arith.wast, at f64x2's 2-lane width.
    # See the NOTICE file for the real vendored/pass counts.
    "simd_f64x2_arith.wast",
    # SIMD widen PR32 (task #211-213): simd_f64x2_cmp.wast -- vendors the
    # dedicated upstream file for f64x2.eq/ne/lt/gt/le/ge (0x47-0x4C),
    # this PR's 6 new opcodes, closing the f64x2 comparison family gap --
    # a direct structural mirror of PR30's simd_f32x4_cmp.wast, at
    # f64x2's 2-lane width. The single BIGGEST directive-count win in
    # this campaign so far -- larger than PR30's simd_f32x4_cmp.wast --
    # see the NOTICE file for the real vendored/pass counts.
    "simd_f64x2_cmp.wast",
    # SIMD widen PR33 (task #214-216): simd_i8x16_sat_arith.wast and
    # simd_i16x8_sat_arith.wast -- vendors the dedicated upstream files
    # for i8x16.add_sat_s/_u/.sub_sat_s/_u (0x6F/0x70/0x72/0x73) and
    # i16x8.add_sat_s/_u/.sub_sat_s/_u (0x8F/0x90/0x92/0x93), this PR's 8
    # new opcodes -- the saturating integer add/sub family, simpler than
    # the float trunc_sat corpus files already vendored (no NaN/infinity
    # edge cases, just compute-then-clamp on integer results). See the
    # NOTICE file for the real vendored/pass counts.
    "simd_i8x16_sat_arith.wast",
    "simd_i16x8_sat_arith.wast",
    # SIMD widen PR34 (task #217-219): simd_f32x4.wast and
    # simd_f32x4_pmin_pmax.wast -- vendors the dedicated upstream files
    # for f32x4.max (0xE9), f32x4.pmin (0xEA), f32x4.pmax (0xEB), this
    # PR's 3 new opcodes, closing the f32x4 arithmetic family (abs/mul/
    # min in PR19, neg/sqrt/add/sub/div in PR29, this PR's max/pmin/pmax).
    # `simd_f32x4.wast` is the upstream corpus's general f32x4 smoke-test
    # file (covers all of abs/neg/sqrt/add/sub/mul/div/min/max, already-
    # implemented ops included, plus this PR's new ones); the deliberately
    # DIFFERENT, deliberately SIMPLER "pseudo-min"/"pseudo-max" semantics
    # (a plain IEEE-754 `<`-based conditional select, NOT the same code
    # path as `min`/`max`'s NaN canonicalization -- see wasm-opcodes'
    # `SimdOpKind::PminF32x4`/`PmaxF32x4` doc comments) get their own
    # dedicated, much larger corpus file, `simd_f32x4_pmin_pmax.wast` --
    # together the best directive-per-opcode ratio in this campaign so
    # far. See the NOTICE file for the real vendored/pass counts.
    "simd_f32x4.wast",
    "simd_f32x4_pmin_pmax.wast",
    # SIMD widen PR35 (task #220-222): simd_f64x2.wast and
    # simd_f64x2_pmin_pmax.wast -- vendors the dedicated upstream files
    # for f64x2.abs (0xEC), f64x2.min (0xF4), f64x2.max (0xF5),
    # f64x2.pmin (0xF6), f64x2.pmax (0xF7), this PR's 5 new opcodes,
    # closing the f64x2 arithmetic family (neg/sqrt/add/sub/mul/div in
    # PR31, this PR's abs/min/max/pmin/pmax) -- a direct structural
    # mirror of PR34's f32x4 closure, at f64x2's 2-lane width, plus
    # `abs` (f32x4.abs already existed since PR19; f64x2.abs did not).
    # `simd_f64x2.wast` is the upstream corpus's general f64x2
    # smoke-test file (covers all of neg/sqrt/add/sub/mul/div/min/max,
    # already-implemented ops included, plus this PR's abs/min/max); the
    # deliberately DIFFERENT, deliberately SIMPLER "pseudo-min"/
    # "pseudo-max" semantics (a plain IEEE-754 `<`-based conditional
    # select, NOT the same code path as `min`/`max`'s NaN
    # canonicalization -- see wasm-opcodes' `SimdOpKind::PminF64x2`/
    # `PmaxF64x2` doc comments) get their own dedicated, much larger
    # corpus file, `simd_f64x2_pmin_pmax.wast` -- same split as PR34's
    # f32x4 pair. See the NOTICE file for the real vendored/pass counts.
    "simd_f64x2.wast",
    "simd_f64x2_pmin_pmax.wast",
    # SIMD widen PR36 (task #223-225): simd_int_to_int_extend.wast --
    # vendors the dedicated upstream file for i64x2.extend_low/
    # high_i32x4_s/_u (0xC7-0xCA), this PR's 4 new opcodes, completing
    # the THIRD and FINAL rung of the "extend" family (i16x8-from-i8x16
    # and i32x4-from-i16x8 both landed opcode-only in PR26; this
    # completes i64x2-from-i32x4). Unlike PR26/PR27/PR28's split
    # simd_conversions.wast handling, this single upstream file already
    # bundles ALL THREE extend rungs together in one `(module ...)` --
    # so it exercises PR26's already-implemented i16x8/i32x4 opcodes too,
    # not just this PR's new i64x2 ones. See the NOTICE file for the
    # real vendored/pass counts.
    "simd_int_to_int_extend.wast",
    # SIMD widen PR37 (task #226-228): simd_lane.wast -- vendors the
    # dedicated upstream file for the remaining extract_lane/replace_lane
    # family members across i16x8/i32x4/i64x2/f32x4/f64x2 (i8x16's own
    # trio and i32x4.extract_lane already existed from SIMD PR1b-2/PR18),
    # this PR's 10 new opcodes, CLOSING the extract_lane/replace_lane
    # family across all six SIMD vector shapes. This single upstream file
    # bundles ALL SIX shapes' extract_lane/replace_lane opcodes together
    # in one `(module ...)` set, so it exercises the pre-existing i8x16/
    # i32x4 lane ops too, not just this PR's new ones -- and it has
    # significant `assert_invalid` coverage for out-of-range lane
    # indices, which this PR also promotes from a runtime-only bounds
    # check to a genuine validation-time rejection (see
    # `wasm-validator/src/type_check.rs`'s `read_lane_index` helper) so
    # those directives are graded as real passes, not
    # `not_yet_supported`. See the NOTICE file for the real vendored/pass
    # counts.
    "simd_lane.wast",
    # SIMD widen PR39: simd_f32x4_rounding.wast and
    # simd_f64x2_rounding.wast -- vendors the dedicated upstream files for
    # the ceil/floor/trunc/nearest "rounding" family across both shapes,
    # this PR's 8 new opcodes (f32x4.ceil/floor/trunc/nearest, f64x2.ceil/
    # floor/trunc/nearest). Verified NOT already vendored/implemented as
    # of PR38's i8x16.shuffle -- this was one of the two open fronts left
    # after the lane-immediate family closed out (the other, the
    # `load_extend`/`load_splat`/`load_zero`/`load{8,16,32,64}_lane`/
    # `store{8,16,32,64}_lane` memory-access family, is deferred to a
    # later PR: it introduces new instruction SHAPES, not just new
    # arithmetic). Both files are self-contained (each covers exactly one
    # vector shape's 4 rounding ops), have real `assert_return` coverage
    # over boundary/special float values plus `assert_invalid`/
    # `assert_malformed` type-mismatch and unknown-operator coverage. See
    # the NOTICE file for the real vendored/pass counts.
    "simd_f32x4_rounding.wast",
    "simd_f64x2_rounding.wast",
    # SIMD PR40: simd_load_splat.wast -- v128.load8_splat/load16_splat/
    # load32_splat/load64_splat (sub-opcodes 0x07-0x0A), the FIRST opcodes
    # in this table that fuse a real linear-memory read with a lane
    # broadcast in one instruction (previously `v128.load`/`v128.store`
    # only moved raw bytes with no lane reinterpretation, and `*.splat`
    # only broadcast an already-on-stack scalar with no memory access).
    # First bite into the wider load-extend/splat/zero/lane family opened
    # by this PR's scoping pass -- see this file's own doc comment.
    "simd_load_splat.wast",
    # SIMD PR41: simd_load_zero.wast -- v128.load32_zero/load64_zero
    # (sub-opcodes 0x5C/0x5D), same "load then fill a v128" shape as
    # `simd_load_splat.wast` above, but ZEROES the non-loaded lanes
    # instead of repeating the loaded value. Second bite into the wider
    # load_extend/load_splat/load_zero/load{8,16,32,64}_lane/
    # store{8,16,32,64}_lane memory-access family PR39 deferred and PR40
    # opened -- the simplest remaining piece (still the plain "pop i32,
    # push v128" type signature, no new instruction SHAPE needed, unlike
    # the lane-load/store family which needs a v128 operand input PLUS a
    # lane-index immediate PLUS a memarg all at once).
    "simd_load_zero.wast",
    # SIMD PR42: simd_load_extend.wast -- v128.load8x8_s/_u,
    # v128.load16x4_s/_u, v128.load32x2_s/_u (sub-opcodes 0x01-0x06), the
    # FIRST opcodes in this family that widen EACH loaded lane
    # independently (sign-extending for `_s`, zero-extending for `_u`)
    # rather than broadcasting one value (`simd_load_splat.wast`) or
    # zero-filling the unused lanes (`simd_load_zero.wast`). Third and
    # final bite into the wider load_extend/load_splat/load_zero/
    # load{8,16,32,64}_lane/store{8,16,32,64}_lane memory-access family
    # PR39 deferred and PR40/PR41 opened -- the lane-load/store family
    # (still not implemented) needs its own new instruction SHAPE (a v128
    # operand input PLUS a lane-index immediate PLUS a memarg all at
    # once), unlike this file, which stays the plain "pop i32, push v128"
    # type signature PR40/PR41 already established.
    "simd_load_extend.wast",
    # SIMD PR43: simd_align.wast -- alignment-hint coverage (align=1/2/
    # 4/8/16, valid/invalid/malformed) for `v128.load`/`v128.store` plus
    # the entire load_splat/load_zero/load_extend family this campaign
    # already landed (PR15/PR39-42). Adds ZERO new opcodes -- every
    # instruction this file exercises (`v128.load`, `v128.store`,
    # `v128.load{8,16,32,64}_splat`, `v128.load{8x8,16x4,32x2}_{s,u}`) was
    # already implemented before this PR. The remaining piece of the
    # load-extend/splat/zero/lane family epic is the
    # load{8,16,32,64}_lane/store{8,16,32,64}_lane shape (a v128 operand
    # PLUS a lane-index immediate PLUS a memarg all at once) -- this file
    # needs none of that, so it slots in now as the simplest possible
    # next bite, same "vendor a corpus file, zero new opcodes" shape as
    # earlier zero-opcode PRs in this campaign. See the NOTICE file for
    # the real vendored/pass counts.
    "simd_align.wast",
]

# Reference-types/threads-proposal files whose UPSTREAM path lives under
# `proposals/<name>/` rather than the repo root every MVP-core file above
# is fetched from. Vendored FLAT under `testsuite/` anyway, like every
# other file here (`wasm_conformance_report`'s file discovery is a plain
# `fs::read_dir` over that one directory, no subdirectory awareness) --
# only the fetch SOURCE path differs, not where the file lands locally.
PROPOSAL_FILES = {
    # WASM18 (plain atomic load/store/RMW/cmpxchg + fence). memory.atomic.
    # notify/wait32/wait64, also in this same file, are NotYetSupported --
    # meaningless without real threads, see code/specs/
    # W09-wasm-atomics-plain.md.
    "atomic.wast": "proposals/threads/atomic.wast",
}

# The corpus itself is Apache-2.0 licensed; vendor the license verbatim
# alongside the fixture files it covers.
EXTRA_FILES = ["LICENSE"]


def raw_url(path: str) -> str:
    return f"https://raw.githubusercontent.com/{REPO}/{PINNED_SHA}/{path}"


def fetch(path: str, dest: Path) -> None:
    with urllib.request.urlopen(raw_url(path)) as response:
        data = response.read()
    dest.write_bytes(data)


def main() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for name in TESTSUITE_FILES + EXTRA_FILES:
        dest = OUTPUT_DIR / name
        print(f"fetching {name} @ {PINNED_SHA[:12]} -> {dest.relative_to(FIXTURES_DIR)}")
        fetch(name, dest)
    for local_name, upstream_path in PROPOSAL_FILES.items():
        dest = OUTPUT_DIR / local_name
        print(f"fetching {local_name} (upstream: {upstream_path}) @ {PINNED_SHA[:12]} -> {dest.relative_to(FIXTURES_DIR)}")
        fetch(upstream_path, dest)
    total = len(TESTSUITE_FILES) + len(PROPOSAL_FILES)
    print(f"done: {total} testsuite files + {len(EXTRA_FILES)} extra file(s)")


if __name__ == "__main__":
    main()
