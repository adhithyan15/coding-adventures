# AOT00-T7 — array reference-tracing fix (LANG-FULL E5, LLVM)

> Status: complete for LLVM; native backends (aarch64/x86_64) explicitly
> out of scope this round, tracked as a follow-up (§4). Closes a gap
> documented (but deliberately left open) by the LLVM `alloc_bytes`/
> `alloc_array` GC-tracking fix (`iir-to-llvm` `lower_alloc_array`'s own doc
> comment).
>
> **Revision note**: an earlier draft of this fix applied the new
> reference-tracing allocator *unconditionally*, on all three backends, on
> the theory that `find_header`-validated over-tracing is "always sound,
> just conservative." A security review caught that this is false against
> the codebase's compacting collector (§2.2) — the design below is the
> corrected one. The original unconditional design never shipped past this
> worktree.

## 1. The confirmed bug

`alloc_array` (LANG-FULL E5 — bounds-checked static arrays) allocates a
single block laid out as `[length header][elem 0][elem 1]…`. Once that
block's own allocation became GC-tracked (a prior round in this same "Twig
GC completion" arc), it was registered under the same no-ref `HeapKind`
`alloc_bytes`/runtime strings use: `__gc_register_kind(NULL, 0)` — an empty
field map, meaning the collector's precise tracer scans **none** of the
block's payload for pointers.

This is correct for a genuinely scalar element (`i64`/`f64`), but an
array's element type can also be `str`, `any`, `symbol`, or `ref<T>` — every
one of these holds a GC **reference** (a heap handle), not an inline scalar.
`algol-iir-compiler`'s `string array` feature already emits exactly this
shape (`array<str>`). Confirmed directly (`gc-core/src/flat_heap.rs`,
`array_registered_under_no_ref_kind_loses_elements_only_reachable_through_it`,
using explicit deterministic roots — no conservative-stack noise): a string
reachable **only** via such an array element is invisible to the collector
and gets reclaimed, corrupting the array's slot to a now-dangling handle —
even though the array's own block correctly survives.

Confirmed to affect all three backends that support GC-tracked arrays
(`iir-to-llvm`, `aarch64-backend`, `x86_64-backend`) equally, since all
three called the same no-ref `__twig_alloc_bytes` for `alloc_array` with no
element-type distinction.

## 2. Fix: a dedicated reference-tracing allocator, applied CONDITIONALLY on LLVM only

`twig_runtime.c` gains `__twig_alloc_ref_array_bytes(int64_t n)`, mirroring
`__twig_alloc_bytes`'s lazy-registration shape but registering under
`__gc_register_ref_array_kind(NULL, 0, 8)` instead of `__gc_register_kind
(NULL, 0)` — `tail_from = 8` skips the block's own 8-byte length header, so
every aligned 8-byte word from offset 8 onward (i.e. every element slot) is
traced as a reference.

`iir-to-llvm::lower_alloc_array` picks between the two allocators **per
array**, based on the *original* IIR element type (`array_elem_type(&instr
.type_hint)`, read *before* `array_elem_llvm`/`llvm_type_for` collapse it to
a plain LLVM type): a new `elem_is_gc_reference` helper matches `"str"`,
`"any"`, `"ref<any>"`, `"symbol"`, and any `"ref<Lispy...>"` — the exact set
`llvm_type_for` collapses to `"i64"` for non-scalar reasons. A
reference-typed element allocates via `__twig_alloc_ref_array_bytes`; a
scalar element (`i64`/`f64`/`bool`/etc.) keeps allocating via the original
no-ref `__twig_alloc_bytes`.

**The native backends (aarch64/x86_64) do NOT get this fix in this round**
and continue calling `__twig_alloc_bytes` unconditionally, unchanged from
before this round started. The AOT specialiser collapses `array<T>`'s
result type to `any` before native codegen ever sees `alloc_array`
(confirmed directly in each backend's own lowering comment), so the element
type is genuinely unavailable there — there is no way to pick a precise
allocator per-array on these two backends without first plumbing the
original element type through the specialiser (§4).

### 2.1 Why not "trace every array unconditionally, everywhere" (the original design)

The first draft of this fix reasoned: `FlatHeap::mark_word` resolves every
traced "reference" word through `find_header` before treating it as live —
a genuinely scalar value that happens to look like a heap address is simply
ignored (`find_header` returns null) rather than followed. Applying the
reference-tracing allocator to *every* array, scalar or not, therefore
seemed always sound against the mark/sweep collector, just conservative —
and since native backends can't tell the element type apart anyway, using
one allocator unconditionally on ALL THREE backends looked like the
simplest design that kept behavior identical everywhere.

### 2.2 The compaction-corruption bug this missed

That reasoning is correct for `FlatHeap`'s non-moving mark/sweep path, but
this codebase's collector also has a real, live, EXPOSED compacting/moving
mode: `FlatHeap::collect_compacting`, wired to the `gc_collect_compacting`
builtin and callable directly from compiled Twig source on both native
backends. A security review of the original unconditional design caught
that over-tracing is NOT safe there:

- During compaction planning, `classify_mobility`/`precise_children` treats
  **every** traced word of a ref-array-kind object as a candidate reference
  edge, resolved via `find_header` exactly like the mark/sweep path.
- If a genuinely scalar array element's bit pattern coincidentally equals
  another live object's exact base address, `find_header` succeeds (it *is*
  a real, live, movable object — just not one this array actually points
  to) and that object becomes part of the array's "precise reachable" set.
- If that object is relocated during this compaction cycle,
  `fixup_ref_fields` walks the array's tail words afterward and calls
  `forwarded(word, forward_map)` on each one. The scalar word's bit pattern
  matches the moved object's OLD address — a genuine forward-map hit — so
  `fixup_ref_fields` OVERWRITES it in place with the new (forwarded)
  address.
- The array's scalar element is now silently, permanently corrupted: a
  different bit pattern than what the program stored, with no error, no
  panic, no signal anything went wrong.

This is qualitatively different from "harmless over-retention" — it is
data corruption, and it directly violates `gc-core-capi`'s own documented
layout contract for `register_ref_array_kind` ("a packed array of unboxed
values must box them, or use `__gc_register_kind` / kind 0 instead").
Unconditional application across all backends was reverted in favor of the
conditional, LLVM-only design in §2.

## 3. Verification

- **`gc-core/src/flat_heap.rs`**:
  `array_registered_under_no_ref_kind_loses_elements_only_reachable_through_it`
  reproduces the original bug (§1) deterministically (explicit roots, no
  conservative-stack noise) — builds the identical 4-slot array as the
  existing `ref_array_traces_elements_precisely` precedent test, but
  registers the array itself under the OLD no-ref kind instead of
  `register_ref_array_kind`, and confirms all four elements (reachable only
  through the array) are wrongly reclaimed. This remains the primary
  regression proof for the underlying primitive-level bug, independent of
  which backends apply the fix.
- **Codegen-shape tests** (deterministic, no execution needed) on
  `iir-to-llvm/tests/test_backend.rs` prove the CONDITIONAL split:
  `array_ops_emit_twig_alloc_bytes_trap_and_gep` proves an `array<i64>`
  (scalar) still allocates via the plain `__twig_alloc_bytes` and never
  touches the ref-array allocator;
  `array_of_str_elements_emits_twig_alloc_ref_array_bytes` proves an
  `array<str>` (reference-typed) allocates via
  `__twig_alloc_ref_array_bytes`.
- **`aarch64-backend`/`x86_64-backend`** — no functional change from before
  this round; both keep calling `__twig_alloc_bytes` unconditionally for
  `alloc_array`, and their existing test suites (unchanged) continue to
  pass.
- **`lang-aot/tests/array_ref_tracing.rs`** — a functional (not
  reclamation-proving; see its own doc comment for why three earlier
  attempts to prove reclamation end-to-end through a compiled-and-run
  program were each found to pass with AND without the fix, due to
  gc-core's conservative stack scan masking the bug via unrelated
  machine-stack noise) integration smoke test: `array<str>` +
  `alloc_array`/`array_set`/`array_get` + real GC allocation pressure
  (70,000 throwaway allocations, crossing `INITIAL_THRESHOLD`) still
  compiles, links, and runs correctly through the real LLVM pipeline. Since
  `str` is a reference-typed element, this continues to exercise the
  `__twig_alloc_ref_array_bytes` path under the corrected conditional logic.
- **`lang-aot/tests/lang_matrix.rs`** — a real, load-bearing test-harness
  bug found by running the FULL matrix (not just the new test): the
  LLVM-column runtime-linking heuristic (`uses_gc_runtime`, string-matches
  the emitted LLVM IR for known runtime-symbol references) checked for
  `@__twig_alloc_bytes` but not the new `@__twig_alloc_ref_array_bytes`, so
  a program using a reference-typed array silently linked WITHOUT
  `twig_runtime.c` and failed with an undefined-symbol error, misreported by
  the harness as "did not complete" rather than a clear link failure — this
  was fixed once and remains correct under the conditional design (LLVM
  still emits `@__twig_alloc_ref_array_bytes` whenever a reference-typed
  array appears, so the heuristic must still recognize it). Confirmed by
  re-running the full matrix after the fix: back to the same 5
  pre-existing, unrelated CLR/JVM/LLVM toolchain failures this session's
  `babysit-pr` runs already treat as known/ignored.
- Full `gc-core`/`gc-core-capi`/`iir-to-llvm`/`aarch64-backend`/
  `x86_64-backend`/`twig-aot`/`lang-aot` suites stay green.
- `cargo clippy --all-targets -- -D warnings` clean across all touched
  crates.

## 4. Explicitly out of scope / follow-up

- **Native-backend (aarch64/x86_64) precise tracing for reference-typed
  arrays.** These two backends keep the narrower, pre-existing
  dangling-reference bug from §1: a `str`/`any`/`ref<T>` array element on
  native codegen is still invisible to the collector. Fixing this requires
  plumbing the original (pre-specialiser) element type through to
  `alloc_array`'s native lowering — the AOT specialiser currently erases it
  to `any` before native codegen ever sees the instruction. This is a
  materially larger change (specialiser surface, not just the two
  backends' `alloc_array` lowering) and is deliberately not attempted in
  this round.
- Compaction/relocation coverage for ref-array-kind objects generally — the
  existing `ref_array_relocates_under_compaction_vs_pinned_conservative_twin`
  precedent already covers the *mechanism* generically; this round doesn't
  add new compaction-specific coverage for `alloc_array` specifically, since
  LLVM codegen doesn't yet drive a compacting collection cycle for E5
  arrays in practice — the corrected §2 design was chosen specifically
  *because* an uncontrolled unconditional fix would have been unsound the
  moment it did.
