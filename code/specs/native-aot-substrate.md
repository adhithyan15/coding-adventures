# Native AOT Substrate — TWIG-GC + TWIG-ROM

**Status**: PR-1 (TWIG-GC) in progress  
**Implements**: Full GraalVM / .NET NativeAOT–style runtime substrate for every
language that compiles to IIR and targets the native AOT backend.

---

## Motivation

GraalVM's SubstrateVM and .NET's NativeAOT CoreRT both ship a complete C/C++
runtime beneath the JIT: garbage collector, object model, exception tables,
closure/delegate heap, dynamic string allocation.  Without these, any language
that needs heap-allocated values (Lisp cons cells, closures, boxed integers,
runtime strings) either leaks or crashes under long-running AOT workloads.

The IIR ecosystem already has:

- `__twig_alloc_bytes` — `calloc`-based allocator that NEVER frees.
- `lispy_runtime.c` — cons cells via `calloc(1, 16)`, also leaks.
- `aarch64-backend` `alloc` op — hardcoded to 16 bytes, calls `alloc_bytes`.
- IIR opcodes `alloc`, `field_store`, `field_load`, `is_null`, `safepoint`,
  `alloc_closure`, `call_closure` — only the first four are implemented in
  the backend; the rest return `UnsupportedOp`.

This spec defines the **three-layer native AOT substrate** that closes those
gaps, one PR per layer.

---

## Layer overview

| Layer | Name | C file(s) | Status |
|-------|------|-----------|--------|
| 1 | **TWIG-GC** — conservative mark-and-sweep garbage collector | `twig_gc.c` | **PR-1** |
| 2 | **TWIG-ROM** — runtime object model (type-tagged header) | `twig_rom.h` | PR-2 |
| 3 | **TWIG-CLOSURES** — alloc_closure + call_closure lowering | `twig_closure.c` + backend | PR-3 |

Follow-up PRs (not in this spec):
- PR-4: setjmp/longjmp exception model for IIR `throw`/`catch`
- PR-5: runtime string creation (dynamic `LangString` from IIR)
- PR-6: x86_64-backend parity (calls, globals, I/O, f64 division)
- PR-7: LANG-STR-RT v2 (type tag at offset 0, TWIG-ROM compatible)

---

## Layer 1 — TWIG-GC

### Design goals

1. **Conservative**: no GC maps required. Every stack word that looks like a
   managed pointer is treated as a root. The compiler does not need to generate
   GC maps or safe-point metadata.
2. **Portable**: compiles with any C99 compiler. No POSIX extensions beyond
   `pthread_self()` / `pthread_getattr_np()` for stack-base detection.
3. **NaN-box compatible**: Lispy heap pointers are stored as `(ptr | 0x7)`.
   The GC scans both `word` and `(word & ~0x7)` to find managed allocations.
4. **Adaptive threshold**: starts at 1 MB; doubles when > 50% of heap is live
   after a collection; halves otherwise (floor: 1 MB).

### Heap object layout

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ gc_header_t  (32 bytes)                                         │
  │  offset  0: next   (8 bytes) — linked list of all objects       │
  │  offset  8: size   (8 bytes) — payload size in bytes            │
  │  offset 16: marked (1 byte)  — mark bit (set during mark phase) │
  │  offset 17: _pad   (15 bytes)— pad to 32 bytes total            │
  ├─────────────────────────────────────────────────────────────────┤
  │ user payload  (size bytes, 16-byte aligned because header = 32) │
  └─────────────────────────────────────────────────────────────────┘
```

The pointer returned to the caller points to the **user payload**, not the
header.  The collector locates the header by subtracting `sizeof(gc_header_t)`.

### Public API

```c
/* Allocate n zero-initialised bytes on the GC heap.  Triggers a collection
 * when total live bytes exceed gc_threshold.  Returns 0 on OOM. */
int64_t __twig_gc_alloc(int64_t n);

/* Force an immediate collection cycle.  Called by safepoint lowering
 * (IIR `safepoint` → BL __twig_gc_safepoint). */
void __twig_gc_collect(void);

/* Safepoint helper — called periodically by running programs to allow the
 * GC to collect when the threshold is exceeded. */
void __twig_gc_safepoint(void);
```

### Collection algorithm

```
collect():
  flush_registers()           ← setjmp into a local jmp_buf
  mark_stack_roots()          ← scan [sp .. stack_base], both raw and untagged
  sweep_dead()                ← walk linked list, free unmarked, clear marks
  update_threshold()          ← adapt threshold based on live/dead ratio
```

**Mark phase** — iterative BFS over a fixed 4096-entry mark stack:

1. Scan the C stack from the current stack pointer to the platform-detected
   stack base.
2. For each stack word `w`:
   - Check if `w` points into a managed allocation (binary-search the sorted
     object table, or walk the linked list).
   - Also check `w & ~0x7` (strips NaN-box tag) for the same.
   - If a managed pointer is found, push it onto the mark stack.
3. Drain the mark stack: pop a pointer, mark its header, scan the payload for
   further managed pointers (same binary-search / linked-list check), push
   unmarked ones.

**Sweep phase** — walk `gc_all_objects` linked list:
- Marked → clear mark bit, advance.
- Unmarked → unlink and `free`.

**Stack-base detection** (platform):
- **macOS**: `pthread_get_stackaddr_np(pthread_self())`
- **Linux**: `pthread_getattr_np` + `pthread_attr_getstack`
- **Windows x64**: `__readgsqword(0x08)` (TEB.StackBase, 64-bit only)

### Integration points

1. **`lispy_runtime.c`** — `__twig_lispy_cons` replaces `calloc(1, 16)` with
   `__twig_gc_alloc(16)`.

2. **`aarch64-backend`** — `alloc` op:
   - Reads size from `srcs[0]` (IIR constant) instead of hardcoding 16.
   - Calls `__twig_gc_alloc` instead of `__twig_alloc_bytes`.

3. **`aarch64-backend` V1_BUILTINS** — adds `gc_alloc` (1 arg, returns) and
   `gc_safepoint` (0 args, no return) so frontends can emit
   `call_builtin "gc_alloc"` directly.

4. **`twig-aot/build.rs`** — adds `twig_gc.c` to the `cc::Build`.

5. **IIR `safepoint` op** — lowered to `BL __twig_gc_safepoint` in the
   aarch64-backend (was previously `UnsupportedOp`).

### What TWIG-GC does NOT do (deferred to PR-2+)

- Type-tagged headers (TWIG-ROM) — the GC header has no type pointer; all
  objects are scanned conservatively without type metadata.
- Precise evacuation / compaction — pure mark-and-sweep only.
- Thread safety — all collections are single-threaded (matches V1's
  single-threaded AOT execution model).
- Finalizers — no destructor callbacks.

---

## Layer 2 — TWIG-ROM (deferred to PR-2)

Every managed heap object gains a `twig_type_t *` at offset 0 of its payload
(offset 32 from the gc_header start).  The GC can then scan the type-directed
field table instead of full conservative scanning.

Layout:

```
  gc_header_t  (32 bytes)
  twig_type_t* (8 bytes, offset 32) — TWIG-ROM type pointer
  fields...
```

LANG-STR-RT v2 adopts this layout, making strings ROM-compatible.

---

## Layer 3 — TWIG-CLOSURES (deferred to PR-3)

IIR `alloc_closure fn_name, cap0, cap1, ...` allocates:

```
  gc_header_t    (32 bytes)
  fn_ptr: i64    (8 bytes)  — pointer to native code for the lambda
  n_caps: i64    (8 bytes)  — number of captured values
  caps[0..n]:    (8 * n bytes) — captured values as raw i64 words
```

IIR `call_closure closure, arg0, arg1, ...` lowers to:

1. Load `fn_ptr` from `closure[0]`
2. Load each captured value from `closure[2+i]`
3. Place captures + user args in argument registers (AAPCS64)
4. `BLR fn_ptr`

The lambda's ABI always takes the closure pointer as its first argument
(implicit self), so `call_closure` just forwards the closure + user args
without needing to know n_caps at the call site.

---

## Verification

Each layer ships with:
1. A unit test in `twig-aot` that exercises the new runtime function directly
   (via the `cc`-linked archive available in `cargo test`).
2. An integration test in `lang-aot` that runs a program exercising the feature
   end-to-end through the AOT compiler.

TWIG-GC unit tests:
- `gc_alloc_returns_aligned_pointer` — result % 8 == 0
- `gc_collect_frees_unreachable` — allocate 100 objects, drop refs, collect,
  verify live count == 0
- `gc_roots_retained_on_stack` — keep a pointer in a local variable, collect,
  verify still alive
- `gc_threshold_adapts` — allocate past threshold, verify collection triggered
