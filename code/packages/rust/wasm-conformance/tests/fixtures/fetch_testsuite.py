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
    # (SIMD PR1a/PR1b, code/specs/W13-wasm-simd-v128-first-slice.md) only
    # covers 5 opcodes (v128.const/i32x4.splat/add/eq/extract_lane), so
    # only the narrowest real corpus file is vendored: `simd_const.wast`
    # tests v128.const's OWN literal syntax across all 6 shapes (this
    # repo's wast-parser already handles all 6, SIMD PR1b-2/1b-3) and is
    # almost entirely gradeable already; its one instruction beyond this
    # slice (a single `i64x2.add` line) now grades NotYetSupported for
    # just that one directive rather than blocking the whole file (W14,
    # code/specs/W14-wasm-conformance-lazy-module-build.md) -- the first
    # real corpus file this repo can vendor from a post-MVP proposal.
    # The other 3 root-level simd_*.wast files (simd_splat.wast,
    # simd_i32x4_arith.wast, simd_i32x4_cmp.wast) each reference many
    # more unsupported opcode families and are deferred to a future PR
    # once coverage widens further (task #76's logged follow-up).
    "simd_const.wast",
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
