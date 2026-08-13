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

# Initial slice: WASM 1.0 MVP core only. Deliberately excludes anything
# needing the `spectest` host-import module or heavier module-linking
# semantics, and anything from a post-MVP proposal (SIMD, threads/atomics,
# exceptions, tail calls, GC/reference-types beyond this repo's existing
# narrow slice, memory64, the component model) -- see
# `code/specs/W05-wasm-conformance-harness.md` section 6 for the full
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
    # Calls
    "call.wast",
    "call_indirect.wast",
    "func.wast",
    "func_ptrs.wast",
    "fac.wast",
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
    # Parser self-test
    "select.wast",
    "comments.wast",
]

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
    print(f"done: {len(TESTSUITE_FILES)} testsuite files + {len(EXTRA_FILES)} extra file(s)")


if __name__ == "__main__":
    main()
