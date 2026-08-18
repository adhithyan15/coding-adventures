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
