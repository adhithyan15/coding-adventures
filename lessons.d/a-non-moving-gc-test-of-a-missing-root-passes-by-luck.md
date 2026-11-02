---
category: Testing & coverage
---

# A non-moving GC test of a missing root passes by luck because stack-mapped frame slots not yet written still hold stale copies of the address; prove rooting with compaction plus cell reuse, or with objects parked outside the stack

**What went wrong.** Fixing #18765 (Twig globals were no GC root in twig-aot), the
natural test was "only a global references the pair; run `gc_collect_precise`; reuse
the freed cell; read the global". It passed on origin/main, with the bug present. So
did a 4 MiB allocation-pressure run through the conservative collector. Dumping the
collector's root slots showed why: the stack map names every reference slot of the
reading function at every safepoint (rule R1), frames are not zeroed, and one of
that function's slots that the executed path had not written yet still held the
pair's address, left there by `main`'s earlier call into the allocator, whose frames
occupied the same stack addresses. Stack "scrubbing" calls did not reliably remove
it.

**What worked.**

- A **compacting** collection with immediate cell **reuse**: the stale slot keeps the
  object alive but cannot keep it in place, so the unrooted word is left naming the
  freed from-space copy and the read returns another object's data (exit 51 instead
  of 7). Valgrind memcheck flags it even without reuse.
- For a deterministic non-moving proof, test at the runtime layer with objects
  **parked outside the stack** (here: in a registered block on the Rust heap), with
  an unregistered control, and assert on exact signals (`freed` count of a heap that
  holds only the batch, `__gc_kind_of`). Reading a marker word after a collection
  proves nothing: freed payload bytes are usually left intact.

**Do differently.** When a rooting bug's test passes before the fix, do not trust
it; check whether a stale stack word is the real root before concluding the
mechanism works. Keep such non-moving cases as guards and say so in the test.
