## Unreleased — 2026-09-15 — VM-041 follow-up: Twig BEAM match/union fusion fix, Twig 49/49

Closed the last Twig BEAM gap VM-041's first cut (0.342.0) left deferred:
`match`/`union` (2 rows) failed lowering with `UnsupportedOp { function:
"Some", op: "field_store: found outside of alloc+field_store+field_store
pattern — lower alloc+2×field_store into put_list before reaching the
backend" }`.

**Pinned down the exact shape** (not guessed at): compiling the
`match`/`union` corpus row through `compile_source_to_iir` and dumping the
synthesized `Some` constructor's IIR (a scratch probe, discarded before this
PR) showed the interleaved instruction is a **`box`**, not the `mov` the
prior slice's own note had left unresolved — and it sits between `alloc` and
the FIRST `field_store`, not between the two `field_store`s:
`twig-ir-compiler::emit_union_def`'s per-field cons-cell loop emitted
`alloc`, then `box` (E6d-6b: boxing the field for the tagged backends'
`match`/`unbox` round-trip), THEN the two `field_store`s —
`[alloc, box, field_store, field_store]`. `iir-to-beam`'s cons-cell fusion
only recognizes the three instructions when textually adjacent (a
`[idx]`/`[idx+1]` look-ahead peek, not a general scan), so the interleaved
`box` broke it.

**Fix chosen: reorder `twig-ir-compiler`'s codegen, not `iir-to-beam`'s
fusion.** `box` only reads the field value — it has no data dependency on
the freshly allocated cell register `alloc` produces — so hoisting `box`
above `alloc` changes no semantics on any of the other five backends and
merely restores `[alloc, field_store, field_store]` adjacency. This mirrors
the SAME function's tag/head cons cell a few lines below, which already
computed its `box` before its own `alloc`. Chosen over teaching
`iir-to-beam`'s fusion look-ahead to tolerate interleaved instructions: the
concrete case has exactly one interleaved-instruction shape, so a general
N-instruction-skip scanner in the shared BEAM backend would be strictly more
machinery for the same outcome.

Proven correct, not just "compiles": `twig-ir-compiler`'s new
`union_constructor_alloc_immediately_followed_by_its_two_field_stores` test
asserts the adjacency directly on the emitted IIR (every `alloc
ref<LispyPair>` is immediately followed by its two matching `field_store`s);
`lang-aot`'s new `twig_beam_match_union` test proves the full pipeline —
source → IIR → BEAM bytes → real `erl` execution — for both promoted rows
(`(match (Some 42) …)` = 42, `(match (None) …)` = 42, the second proving
tag-dispatch discrimination, not just the first arm).

`twig-ir-compiler` 0.45.0 → 0.45.1: `emit_union_def`'s per-field cons-cell
loop now emits `box` before `alloc` (was: `alloc` then `box`). Behavior is
identical on every non-BEAM backend (WASM/JVM/CLR/NativeAOT/LLVM/VM/JIT);
this is a pure BEAM-fusion-adjacency fix.

`lang-aot` 0.342.0 → 0.343.0: Twig's remaining 2 `match`/`union` rows
promoted to declare `Beam`, bringing Twig to **49/49** — fully complete on
BEAM. `feature_coverage_doc_counts_match_programs_source`'s Twig tuple
updated `(49, 390)` → `(49, 392)`; `LANG-VM-FEATURE-COVERAGE.md`'s Twig row
and grand-total prose (1663 → 1665) updated to match. New dedicated test
`twig_beam_match_union`, executed against real `erl` before promotion, per
this backlog's "probe before declaring, promote only proven cells"
discipline.

With Twig closed to 49/49, the only non-ALGOL BEAM gap left in
`LANG-VM-NON-ALGOL-BACKLOG.md` is Dartmouth BASIC's remaining 8
design-blocked rows (5 `INPUT` rows on VM-060b, 2 string-array/mixed-`DATA`
rows, `RND` on VM-018's module-global design question) — see the backlog's
top section for the reprioritization.
