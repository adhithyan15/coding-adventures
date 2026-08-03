# AOT00-T7 — array reference-tracing fix (LANG-FULL E5, cross-backend)

> Status: complete. Closes a gap documented (but deliberately left open) by
> the LLVM `alloc_bytes`/`alloc_array` GC-tracking fix (`iir-to-llvm`
> `lower_alloc_array`'s own doc comment), confirmed cross-backend (LLVM,
> aarch64, x86_64) rather than assumed to be LLVM-only.

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

Confirmed to affect **all three** backends that support GC-tracked arrays:
`iir-to-llvm`, `aarch64-backend`, and `x86_64-backend` all called the same
no-ref `__twig_alloc_bytes` for `alloc_array`, with no element-type
distinction.

## 2. Fix: a dedicated reference-tracing allocator, used unconditionally

`twig_runtime.c` gains `__twig_alloc_ref_array_bytes(int64_t n)`, mirroring
`__twig_alloc_bytes`'s lazy-registration shape but registering under
`__gc_register_ref_array_kind(NULL, 0, 8)` instead of `__gc_register_kind
(NULL, 0)` — `tail_from = 8` skips the block's own 8-byte length header, so
every aligned 8-byte word from offset 8 onward (i.e. every element slot) is
traced as a reference.

All three backends' `alloc_array` lowering now calls this new allocator
**unconditionally** — not only when the frontend declares a reference-typed
element:

- On the native backends (`aarch64-backend`/`x86_64-backend`), the element
  type genuinely **isn't available** at this call site: the AOT specialiser
  collapses `array<T>`'s result type to `any` before native codegen ever
  sees it (confirmed directly in each backend's own `alloc_array` lowering
  comment), so there is no way to conditionally pick the no-ref allocator
  only for scalar arrays without risking silently missing a reference-typed
  one.
- On LLVM, the element type *is* available (`array_elem_type(&instr.type_hint)`),
  but `array_elem_llvm`/`llvm_type_for` map several heap-handle types
  (`"str"`, `"any"`, `"symbol"`, `"ref<Lispy...>"`) down to the same `"i64"`
  LLVM type plain integers use — the same confirmed gap. Using the
  reference-tracing allocator unconditionally here too keeps behavior
  identical across all three backends rather than diverging on a
  distinction none of them can make reliably.

**Why "trace every array unconditionally" is sound, not just convenient:**
`FlatHeap::mark_word` resolves every traced "reference" word through
`find_header` before treating it as live — a genuinely scalar `i64`/`f64`
value that happens to look like a heap address is simply ignored (`find_header`
returns null) rather than followed. The only cost of over-tracing a scalar
array is a few extra `find_header` probes per collection; there is no
correctness cost (never under-traces, never corrupts, at most a coincidental
false-positive keeping some unrelated object alive one cycle longer than
strictly necessary) — the same conservative-is-safe principle the
collector's own stack scan already relies on.

## 3. Verification

- **`gc-core/src/flat_heap.rs`**:
  `array_registered_under_no_ref_kind_loses_elements_only_reachable_through_it`
  reproduces the exact bug deterministically (explicit roots, no
  conservative-stack noise) — builds the identical 4-slot array as the
  existing `ref_array_traces_elements_precisely` precedent test, but
  registers the array itself under the OLD no-ref kind instead of
  `register_ref_array_kind`, and confirms all four elements (reachable only
  through the array) are wrongly reclaimed. This is the primary regression
  proof for this round.
- **Codegen-shape tests** (deterministic, no execution needed) on all three
  backends confirm `alloc_array` now relocates/calls
  `__twig_alloc_ref_array_bytes`, never the old `__twig_alloc_bytes`:
  `iir-to-llvm/tests/test_backend.rs`
  (`array_ops_emit_twig_alloc_ref_array_bytes_trap_and_gep`),
  `aarch64-backend`
  (`alloc_array_emits_bl_to_ref_array_runtime_not_plain_alloc_bytes`),
  `x86_64-backend`
  (`alloc_array_emits_call_to_ref_array_runtime_not_plain_alloc_bytes`).
- **`lang-aot/tests/array_ref_tracing.rs`** — a functional (not
  reclamation-proving; see its own doc comment for why three earlier
  attempts to prove reclamation end-to-end through a compiled-and-run
  program were each found to pass with AND without the fix, due to
  gc-core's conservative stack scan masking the bug via unrelated
  machine-stack noise) integration smoke test: `array<str>` +
  `alloc_array`/`array_set`/`array_get` + real GC allocation pressure
  (70,000 throwaway allocations, crossing `INITIAL_THRESHOLD`) still
  compiles, links, and runs correctly through the real LLVM pipeline.
- **`lang-aot/tests/lang_matrix.rs`** — a real, load-bearing test-harness
  bug found by running the FULL matrix (not just the new test): the
  LLVM-column runtime-linking heuristic (`uses_gc_runtime`, string-matches
  the emitted LLVM IR for known runtime-symbol references) checked for
  `@__twig_alloc_bytes` but not the new `@__twig_alloc_ref_array_bytes`, so
  every ALGOL/Twig array program silently linked WITHOUT `twig_runtime.c` and
  failed with an undefined-symbol error, misreported by the harness as
  "did not complete" rather than a clear link failure — 24 matrix tests
  failed until this was fixed. Confirmed by re-running the full matrix
  after the fix: back to the same 5 pre-existing, unrelated CLR/JVM/LLVM
  toolchain failures this session's `babysit-pr` runs already treat as
  known/ignored.
- Full `gc-core`/`gc-core-capi`/`iir-to-llvm`/`aarch64-backend`/
  `x86_64-backend`/`twig-aot`/`lang-aot` suites stay green.
- `cargo clippy --all-targets -- -D warnings` clean across all touched
  crates.

## 4. Explicitly out of scope

- Compaction/relocation for ref-array-kind objects under this fix — the
  existing `ref_array_relocates_under_compaction_vs_pinned_conservative_twin`
  precedent already covers the *mechanism* generically; this round doesn't
  add new compaction-specific coverage for `alloc_array` specifically, since
  `iir-to-llvm`/native codegen doesn't yet drive a compacting collection
  cycle for E5 arrays in practice.
- A precise (element-type-aware) allocator choice on LLVM specifically —
  deliberately not built, to keep behavior identical across all three
  backends (see §2) rather than LLVM alone doing better than what aarch64/
  x86_64 can support.
