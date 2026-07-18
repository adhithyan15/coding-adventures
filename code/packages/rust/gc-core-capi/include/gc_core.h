/*
 * gc_core.h — C ABI for gc-core's flat-native heap.
 * =================================================================
 *
 * Declarations for the symbols exported by `libgc_core_capi.a` (the Rust
 * `gc-core-capi` crate). This archive is LANG16's `gc_runtime_<target>.a`:
 * a native-AOT executable links it so its emitted `alloc` / `field_*` /
 * `safepoint` ops resolve to a real garbage collector. It supersedes the
 * Twig-specific `twig_gc.c` — the same flat mark-and-sweep model, but one
 * generic collector (gc-core) shared by every native consumer.
 *
 * See code/specs/AOT00-T1-precise-gc.md and code/specs/LANG16-gc-core.md.
 *
 * Model
 * -----
 *   __gc_alloc(n) returns a REAL pointer (as int64_t) to n zeroed bytes; the
 *   payload is 16-byte aligned. Compiled code reads/writes it directly at byte
 *   offsets. A returned pointer stays valid until a __gc_collect_roots that
 *   does not root it, or __gc_reset.
 *
 *   Tracing is conservative: each root word and each aligned word inside a
 *   reachable object is treated as a candidate pointer (raw and low-3-bit
 *   tag-stripped, for NaN-boxed heap references). False positives retain a dead
 *   object one extra cycle; a live object is never freed.
 *
 *   Single-threaded (matching twig_gc.c). One process-wide heap.
 */
#ifndef GC_CORE_H
#define GC_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Allocate n zeroed bytes; returns the payload pointer as int64_t, or 0 on
 * n <= 0, size overflow, or allocator failure. */
int64_t __gc_alloc(int64_t n);

/* As __gc_alloc, tagging the object with a HeapKind id (0 = opaque / trace
 * conservatively; a registered kind id enables precise interior tracing). */
int64_t __gc_alloc_kind(int64_t n, uint16_t kind);

/* Register a reference-field map (the byte offsets of an object layout's ref
 * fields) and return a 1-based kind id to pass to __gc_alloc_kind. Objects of
 * that kind are traced PRECISELY — only the mapped offsets are followed — so a
 * look-alike-pointer integer in a non-reference field cannot pin a phantom
 * child. A null list or count <= 0 registers an opaque (no-ref-field) kind;
 * negative offsets are ignored. This is how a frontend teaches the collector
 * its object layouts (records, tuples, Ruby/Python/JS objects). */
int64_t __gc_register_kind(const int64_t *field_offsets, int64_t count);

/* Mark from `count` root words at `roots`, then sweep. Returns objects freed.
 * A null `roots` or count <= 0 means "no roots". */
int64_t __gc_collect_roots(const int64_t *roots, int64_t count);

/* Mark from every candidate pointer in the raw region [base, base+len), then
 * sweep. Returns objects freed. This is the region-scan primitive for rooting
 * from memory the collector must scan itself — a spilled-register block, or the
 * machine call stack between the stack pointer and the thread's stack base. The
 * argument-less native collect/safepoint drop-ins (a follow-up) discover that
 * range and call this. A null `base` or len <= 0 means "no region" → free all. */
int64_t __gc_collect_region(const uint8_t *base, int64_t len);

/* Run a full conservative collection rooted at THIS THREAD'S live C stack and
 * callee-saved registers — no caller-supplied roots. Spills callee-saved
 * registers to the stack, reads the stack pointer, finds the thread's stack
 * base (pthread on macOS/Linux, GetCurrentThreadStackLimits on Windows), and
 * hands [sp, base) to __gc_collect_region. This is the drop-in for the native
 * backend's argument-less collect/safepoint points, where the only roots are
 * whatever the machine is holding. Returns objects freed. Single-threaded. */
int64_t __gc_collect(void);

/* Paced collect: run __gc_collect ONLY if the live set has reached the heap's
 * adaptive threshold; otherwise do nothing. Returns objects freed (0 if no
 * collection ran). Drop-in for twig_gc.c's __twig_gc_safepoint — the native
 * backend calls it at loop back-edges and function entries, and it collects
 * only under memory pressure so a tight allocation loop can't starve the GC.
 * __gc_alloc also runs this same paced collect before allocating. */
int64_t __gc_safepoint(void);

/* Generational write barrier: call whenever the mutator stores a heap reference
 * `child` into a field of heap object `parent` (both payload addresses). If
 * `parent` is old, it is recorded so a later __gc_collect_minor scans it for the
 * young objects it now references. O(1); `child` is never dereferenced. `parent`
 * must be a live GC-object payload (a `parent < 32` is ignored). */
void __gc_write_barrier(int64_t parent, int64_t child);

/* Minor (young-generation-only) collection rooted at this thread's live stack +
 * callee-saved registers — the generational analogue of __gc_collect. Reclaims
 * only young garbage; old objects are never scanned or freed (old->young pointers
 * are reached through the remembered set __gc_write_barrier populates). Returns
 * objects freed. Requires every old->young store to have called the barrier. */
int64_t __gc_collect_minor(void);

/* Live payload bytes. */
int64_t __gc_live_bytes(void);

/* Collections run since process start (or last __gc_reset). */
int64_t __gc_collection_count(void);

/* Drop the whole heap (frees everything) and reset counters. */
void __gc_reset(void);

/* ── Stack-map registry (precise roots) ─────────────────────────────────────
 * Register one compiled function's stack maps so the precise stack walker can
 * turn a return address inside it into the live-reference slots at that PC. Call
 * once per function at image start-up, before any collection.
 *
 * The function occupies the code range [func_start, func_start + func_len). Its
 * num_records safepoint records are passed as PARALLEL FLATTENED arrays, each of
 * length num_records:
 *   pc_offsets[i]   — safepoint offset from func_start (the lookup key)
 *   frame_sizes[i]  — frame size in bytes (walker steps to the caller); may be NULL
 *   callee_masks[i] — bitmask of callee-saved regs holding refs here; may be NULL
 *   slot_counts[i]  — number of reference slots for record i (negative => 0)
 * plus one concatenated slots array read record-by-record through the counts:
 *   slots_flat      — record i owns the next slot_counts[i] entries (FP-relative
 *                     byte offsets, may be negative); may be NULL if all counts 0.
 *
 * Returns the number of records stored (> 0), or 0 if rejected (func_len == 0,
 * func_len > UINT32_MAX (pc_offset is a uint32), num_records <= 0, a required
 * array NULL, the range wraps, or it overlaps an already-registered function).
 * frame_sizes/callee_masks are carried for the walker; resolution uses only
 * pc_offsets + slots. */
int64_t __gc_register_stackmap(uint64_t func_start, uint64_t func_len,
                               int64_t num_records, const uint32_t *pc_offsets,
                               const uint32_t *frame_sizes,
                               const uint16_t *callee_masks,
                               const int32_t *slot_counts,
                               const int32_t *slots_flat);

/* Number of functions currently registered via __gc_register_stackmap. */
int64_t __gc_stackmap_count(void);

/* Drop all registered stack maps. Code maps normally live for the whole process,
 * so this is NOT run by __gc_reset; it is for test isolation / teardown. */
void __gc_stackmap_reset(void);

/* ── twig-compat aliases ────────────────────────────────────────────────────
 * The native-AOT code generators and dynval_runtime.c reference the symbol
 * names the retired twig_gc.c exported. These forward to the __gc_* ABI above
 * (prototypes match twig_gc.c exactly, including the void-returning collect /
 * safepoint). Deletable once the emitters emit the __gc_* names directly. */
int64_t __twig_gc_alloc(int64_t n);
void    __twig_gc_collect(void);
void    __twig_gc_safepoint(void);
int64_t __twig_gc_live_bytes(void);
int64_t __twig_gc_collection_count(void);

#ifdef __cplusplus
}
#endif

#endif /* GC_CORE_H */
