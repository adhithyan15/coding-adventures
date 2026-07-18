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

/* As __gc_alloc, tagging the object with a HeapKind id (for later precise
 * interior tracing; 0 = opaque / trace conservatively). */
int64_t __gc_alloc_kind(int64_t n, uint16_t kind);

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

/* Live payload bytes. */
int64_t __gc_live_bytes(void);

/* Collections run since process start (or last __gc_reset). */
int64_t __gc_collection_count(void);

/* Drop the whole heap (frees everything) and reset counters. */
void __gc_reset(void);

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
