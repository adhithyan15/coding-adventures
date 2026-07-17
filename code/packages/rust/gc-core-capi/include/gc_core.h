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

/* Live payload bytes. */
int64_t __gc_live_bytes(void);

/* Collections run since process start (or last __gc_reset). */
int64_t __gc_collection_count(void);

/* Drop the whole heap (frees everything) and reset counters. */
void __gc_reset(void);

#ifdef __cplusplus
}
#endif

#endif /* GC_CORE_H */
