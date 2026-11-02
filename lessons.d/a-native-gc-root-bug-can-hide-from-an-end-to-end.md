---
category: Compiler / VM / language pipeline
---

# A native GC-root bug can hide from an end-to-end precise-collection test, because the walk scans the collector's own frames conservatively; check the stack-map types instead

**What happened.** LANG83 PR5a had to check whether a native `str` slot is
still a GC root after `aot-core`'s specialiser. `twig-aot` materialises a
string literal as `alloc_bytes buf : i64` plus `mov dest = buf : i64`. Both
types are machine scalars, so `is_gc_root_ty` leaves both slots out of the
stack map, and a precise or compacting collection cannot see a live literal.
The first test kept a literal across `gc_collect_precise`, allocated a
same-sized string and printed the literal. It passed on the buggy code:
the collection freed nothing (`freed = 0`, live bytes unchanged). Two things
hid the bug. A function with no root-typed slot gets no stack-map record, and
the walk scans an unmapped frame conservatively. Once a run-time string made
`main` mapped, the test still passed: the walk scans `[sp, start_fp)` (the
collector's own frames) conservatively, and a stale copy of the handle left
there by an earlier call kept the block alive and pinned.

**Fix.** The literal's `alloc_bytes` is typed `str`; `root_heap_word_copies`
then roots its `mov` alias. The regression test checks the CIR types after
`prepare_module_for_aot` + `aot_specialise` + `root_heap_word_copies`, and that
the x86-64 stack map names a slot. That test fails on the old typing.

**Do differently.** For a GC-rooting question on the native backends, assert
on the root *types* (or the stack-map records) the backend receives, not on
whether a collection happened to free something. If you also write the
end-to-end run, print the collector's `freed` count first: `0` means the
test is not exercising the bug. This is the same trap as the lesson about
making a value unreachable in a callee and then collecting in the caller.
