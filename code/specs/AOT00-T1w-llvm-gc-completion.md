# AOT00-T1w — LLVM's Twig GC gaps closed (capstone: complete)

> **Status: complete.** Part 2 of a 3-part pass driving Twig's garbage
> collector to completion across every backend (Part 1: `vm-core`'s
> `alloc`/`field_store`/`field_load` reroute, `AOT00-T1v`; Part 3: WASM linear
> memory, forthcoming). This part closes LLVM's two confirmed gaps: an
> earlier sub-agent's claim that LLVM-compiled cons cells/records never
> collect at all was **wrong** — verified false by directly reading
> `gc-core-capi`'s allocator and then proving it with a real, running
> end-to-end test — and `alloc_bytes`/`alloc_array` (Brainfuck's byte tape,
> LANG-FULL E5 arrays) called raw, untracked `@calloc`, a genuine, confirmed,
> permanent leak.

## 1. The two claims, checked against the actual code

**Claim: "LLVM never collects."** An earlier investigation this session
concluded LLVM-compiled cons cells/records leak forever, based on reading
`iir-to-llvm`'s own codegen and finding no explicit safepoint/collect call
anywhere. That's true as far as it goes, but incomplete: `lower_alloc`
(`iir-to-llvm/src/lib.rs`) calls `@__twig_gc_alloc`, which routes through
`gc-core-capi`'s `__gc_alloc_kind` (`gc-core-capi/src/lib.rs:106-123`) —
and `__gc_alloc_kind` **already runs a conservative collection before every
single allocation** whenever `FlatHeap::should_collect()` says the live-byte
threshold has been crossed:

```rust
pub extern "C" fn __gc_alloc_kind(n: i64, kind: u16) -> i64 {
    if n <= 0 { return 0; }
    if with_heap(|h| h.should_collect()) {
        unsafe { stack_scan::__gc_collect() };
    }
    with_heap(|h| h.alloc(n as usize, kind) as i64)
}
```

This is a conservative *stack* scan, agnostic to which compiler produced the
currently-executing machine code — it works identically whether that code
came from `aarch64-backend`'s hand-written codegen or `clang`-compiled LLVM
IR. So LLVM-compiled records/cons cells *do* auto-collect, for free, with no
LLVM-side safepoint emission needed — the same mechanism aarch64/x86_64
already rely on.

**Verified, not assumed**: `lang-aot/tests/llvm_gc_completion.rs`'s
`alloc_on_llvm_auto_collects_under_real_allocation_pressure` hand-builds an
IIR module (a loop calling a helper function 70,000 times, each call
allocating one throwaway 16-byte cons cell via `alloc`), compiles it through
the real `iir-to-llvm` lowering, links the actual `gc-core-capi` archive,
and **runs** the resulting binary — 70,000 × 16 bytes ≈ 1.09 MiB, over
`gc_core::flat_heap::INITIAL_THRESHOLD` (1 MiB), so the auto-collect-before-
alloc check must fire from ordinary allocation pressure alone, with no
explicit `gc_collect` call anywhere in the module. The program proves this
by comparing `gc_live_bytes()` against a small bound *inside* the IIR itself
(see §3 for why) and returns that boolean as its exit code.

**Claim: `alloc_bytes`/`alloc_array` leak.** Confirmed true, and fixed.
`lower_alloc_bytes`/`lower_alloc_array` called raw `@calloc` — never freed,
never traced, a genuine permanent leak for Brainfuck's tape and every
LANG-FULL E5 array. Both now call `@__twig_alloc_bytes`
(`twig-aot/runtime/twig_runtime.c:225`) instead — the same GC-tracked
allocator `aarch64-backend`/`x86_64-backend` already use for the identical
ops, and the same one this runtime's own `str_concat`/`str_slice` helpers
already allocate through. `__twig_alloc_bytes` registers its blocks under a
no-ref `HeapKind` (an empty field map — nothing to trace), which is exactly
right here: `array_elem_llvm` only ever accepts scalar/numeric element types
(`i1`/`i8`/`i16`/`i32`/`i64`/`float`/`double`; no `ptr`/`ref` type exists in
that match), so an LLVM-compiled array can never hold a GC reference in the
first place — confirmed by reading the type table, not assumed.

Both call sites' new i64-handle return value is recovered as a `ptr` via
`inttoptr`, the same convention `lower_field_store`/`lower_field_load`
already use to turn `alloc`'s own i64 handle back into a pointer.
`array_get`/`array_set`/`array_len`'s existing handle model (a `ptr` 8 bytes
into the block, past the length header) is otherwise unchanged —
`FlatHeap::find_header` resolves an *interior* address to its enclosing
block, so this offset pointer stays a valid, collectible root exactly like a
base-address one.

## 2. `gc_live_bytes` — a diagnostic builtin, mirroring the native backends

`aarch64-backend`/`x86_64-backend` already expose `gc_live_bytes` (`n_args:
0, returns: true`) as a `call_builtin` name, resolved generically via their
`__twig_{name}` symbol convention. `iir-to-llvm` had no equivalent — added
to `SUPPORTED_BUILTINS`, lowering to `@__twig_gc_live_bytes()`
(`gc-core-capi`'s `twig_compat` module, already linked wherever
`@__twig_gc_alloc` is). This is what makes §1's end-to-end proof possible at
all — without it, there would be no way for a Twig-level (or hand-built IIR)
program to observe the collector's live-byte total and prove reclamation
genuinely happened, as opposed to trusting the C source's own claims.

## 3. A real bug found while writing the proof: exit-code truncation

The first draft of the end-to-end test returned `gc_live_bytes()`'s raw
value as the process exit code. Process exit codes are truncated to their
low 8 bits on POSIX — and the *uncollected* total for this exact test
(70,001 × 16 = 1,120,016 bytes) happens to have low byte `16`, which sits
comfortably inside a naively-chosen "looks small, so it must have collected"
range. A masked exit code cannot distinguish "genuinely collected" from
"never collected, but this particular total's low byte looks small by
coincidence." Fixed by comparing `gc_live_bytes() < threshold` *inside* the
IIR module itself (a `cmp_lt` immediately before `ret`) and returning that
boolean — a comparison result is always exactly 0 or 1, so it survives
8-bit truncation unambiguously. Worth recording: this is a general trap for
any test that returns a large diagnostic value as an exit code rather than a
pre-reduced boolean/small-range result.

Fixing the truncation exposed a second, quieter problem: with the boolean
comparison in place, the test *still* failed. A one-off debug build
(printing the raw value over stdout instead of comparing it) measured the
real post-loop residual at ~71,440 bytes — real reclamation (down from
~1.12 MiB), but nowhere near zero. This is the same "conservative scanning
+ adaptive threshold leaves a legitimate residual, not exactly zero" shape
this session's `vm-core` and WASM struct-heap end-to-end tests both already
hit: `FlatHeap::adapt_threshold` doubles the collection threshold whenever
more than half a cycle's live set appears to survive, and a conservative
stack scan can retain some fraction of already-dead objects as
false-positive pointer-like stack values — so after the first collection
crosses the threshold once, later allocations in the same run may not
trigger a second cycle before the loop ends. The threshold this test
compares against (300,000 bytes) is picked to sit comfortably above the
measured ~71,440 (so it isn't a flaky near-exact bound) while staying far
below the ~1.12 MiB a genuinely non-collecting run would leave live —
proving real, substantial reclamation without demanding an unrealistic
near-zero residual from a conservative collector.

## 4. A test-harness bug found by running the full matrix, not just the new test

`lang-aot/tests/lang_matrix.rs`'s `run_llvm` conditionally links two
overlapping runtimes: a minimal, standalone `PRINT_RUNTIME_C` (originally
`__print_i64`/`__print_str` *plus* standalone reimplementations of
`__twig_input_i64`/`__twig_input_str`/`__twig_str_concat`/`__twig_str_eq`/
`__twig_str_cmp`, for programs simple enough not to need the full
`twig_runtime.c` archive), and the full `dynval_runtime.c` + `gc-core-capi` +
`twig_runtime.c` stack (for programs that need real dynamic/GC support).
These were previously mutually exclusive in practice — until this round's
`alloc_array`/`alloc_bytes` fix made *any* array-using program (ALGOL string
arrays, in particular, once they also called `print`) reference
`@__twig_alloc_bytes`, triggering **both** conditions simultaneously for the
first time. Since both runtimes define the same five `__twig_input_*`/
`__twig_str_*` symbols, this produced a genuine `ld: duplicate symbol` link
failure — caught by running the *full* `lang_matrix` suite (not just the new
unit/e2e tests), which is exactly why that step is part of the standard
verification sweep and not optional.

Fixed by splitting `PRINT_RUNTIME_C` into `PRINT_RUNTIME_C` (just
`__print_i64`/`__print_str`, always safe to link alongside `twig_runtime.c`
since it has no equivalent there) and `MISC_IO_RUNTIME_C` (the five
`__twig_*` functions, linked only when `twig_runtime.c` itself is *not*
already being linked).

## 5. What's explicitly out of scope

- **WASM's linear-memory strings/arrays** — a separate backend, separate
  crate (`iir-to-wasm`), tracked as Part 3 of this pass.
- **A per-kind precise interior trace for LLVM's `alloc`'d objects** — same
  kind-0/conservative boundary `AOT00-T1v` draws for `vm-core`; LLVM shares
  the identical `gc-core-capi` engine, so the same boundary applies
  unchanged.
- **Compaction-awareness in LLVM codegen** — `__gc_alloc_kind`'s auto-collect
  may already relocate objects under `gc-core`'s adaptive policy (shared with
  every other consumer of this allocator); this round didn't need to change
  anything LLVM-side for that, since LLVM never held a raw pointer across a
  potential collection point in a way that would need fixing up (values flow
  as i64 handles, converted to `ptr` immediately before use, not held live
  across a call boundary as a native pointer).

## 6. Tests

- `iir-to-llvm/tests/test_backend.rs`: `alloc_bytes_emits_twig_alloc_bytes_and_declare`,
  `array_ops_emit_twig_alloc_bytes_trap_and_gep` (renamed from their
  `_calloc_` predecessors, updated to assert `@__twig_alloc_bytes` and the
  absence of `@calloc`, plus the `inttoptr` handle-recovery step).
- `lang-aot/tests/llvm_gc_completion.rs` (new file):
  `alloc_on_llvm_auto_collects_under_real_allocation_pressure` — the
  headline end-to-end proof described in §1.
- `lang-aot/tests/lang_matrix.rs`: full suite re-verified green (the
  §4 fix), including every ALGOL array program (scalar and string,
  single- and multi-dimensional, plain and nested-procedure-captured) on the
  LLVM column.
