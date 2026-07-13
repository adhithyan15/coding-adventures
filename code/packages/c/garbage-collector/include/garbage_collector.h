/*
 * garbage_collector.h — mark-and-sweep garbage collector, pure ISO C17.
 * ====================================================================
 *
 * A faithful port of the Rust `garbage-collector` crate — a language-agnostic
 * tracing GC any VM can use.
 *
 * ## The algorithm
 *
 *   1. Mark:  starting from the roots, follow every reference and mark each
 *             reachable object (cycles are handled by the already-marked guard).
 *   2. Sweep: walk the heap; free any object that was not marked.
 *   3. Reset: clear the marks on survivors for the next cycle.
 *
 * Heap objects are `GcObject`s (a cons cell, an interned symbol, or a Lisp
 * closure). Each stores its references as heap addresses; the GC follows them
 * during marking. Roots are `GcValue`s — only address-like values are followed.
 *
 * Addresses are monotonically increasing from 0x10000 (so they never collide
 * with the small integers a program manipulates) and are never reused.
 *
 * ## Ownership
 *
 * `gc_allocate` TAKES OWNERSHIP of the object and returns its address; the GC
 * frees swept objects and, on `gc_free`, everything still live. Roots you build
 * with the `gc_val_*` constructors are owned by you — release string/list roots
 * with `gc_value_free`. A `GcSymbolTable` borrows its backing GC.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef GARBAGE_COLLECTOR_H
#define GARBAGE_COLLECTOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Root values ───────────────────────────────────────────────────────────
 *
 * A runtime value that may or may not be a heap address. The GC scans roots and
 * follows only address-like values (Address, and Int reinterpreted as address).
 */
typedef enum {
    GC_VAL_INT,
    GC_VAL_ADDRESS,
    GC_VAL_STR,
    GC_VAL_BOOL,
    GC_VAL_NIL,
    GC_VAL_LIST
} GcValueKind;

typedef struct GcValue GcValue;
struct GcValue {
    GcValueKind kind;
    union {
        int64_t i;      /* GC_VAL_INT */
        size_t address; /* GC_VAL_ADDRESS */
        char *str;      /* GC_VAL_STR (owned) */
        int b;          /* GC_VAL_BOOL */
        struct {
            GcValue *items; /* owned array */
            size_t n;
        } list; /* GC_VAL_LIST */
    } as;
};

GcValue gc_val_int(int64_t v);
GcValue gc_val_address(size_t addr);
GcValue gc_val_bool(int b);
GcValue gc_val_nil(void);
GcValue gc_val_str(const char *s);                    /* copies `s` */
GcValue gc_val_list(const GcValue *items, size_t n);  /* deep-copies items */
/* Release a string/list value's owned storage (no-op for scalars). */
void gc_value_free(GcValue *v);

/* ── Heap objects ──────────────────────────────────────────────────────────*/

typedef struct GcObject GcObject;

/* Construct heap objects (allocate with gc_allocate, which takes ownership).
 * Return NULL on allocation failure. */
GcObject *gc_cons_new(int64_t car, int64_t cdr);
GcObject *gc_symbol_new(const char *name);
/* A closure capturing `code`, an environment (parallel key/value arrays), and
 * parameter names. Copies all inputs. NULL on OOM. */
GcObject *gc_closure_new(const char *code, const char *const *env_keys,
                         const int64_t *env_vals, size_t n_env,
                         const char *const *params, size_t n_params);
/* Free a standalone object not yet handed to a GC. NULL-safe. */
void gc_object_free(GcObject *obj);

/* Human-readable type name: "ConsCell", "Symbol", or "LispClosure". */
const char *gc_object_type_name(const GcObject *obj);

/* The heap addresses this object references (what the GC follows when marking).
 * Returns a malloc'd array of `*n_out` addresses (NULL when none/on OOM); the
 * caller frees it. */
size_t *gc_object_references(const GcObject *obj, size_t *n_out);

/* ── The collector ─────────────────────────────────────────────────────────*/

typedef struct {
    size_t total_allocations;
    size_t total_collections;
    size_t total_freed;
    size_t heap_size;
} GcStats;

typedef struct GcHeap GcHeap;

/* Create / destroy a mark-and-sweep GC. gc_free releases all live objects. */
GcHeap *gc_new(void);
void gc_free(GcHeap *gc);

/* Allocate `obj` (takes ownership) and return its heap address. On allocation
 * failure the object is freed and 0 is returned (0 is never a valid address). */
size_t gc_allocate(GcHeap *gc, GcObject *obj);
/* Look up a live object by address, or NULL. */
const GcObject *gc_deref(const GcHeap *gc, size_t address);
/* Run one collection over `roots`; returns the number of objects freed. */
size_t gc_collect(GcHeap *gc, const GcValue *roots, size_t n_roots);
/* Number of live objects. */
size_t gc_heap_size(const GcHeap *gc);
/* Does `address` point at a live object? */
int gc_is_valid_address(const GcHeap *gc, size_t address);
/* Introspection counters. */
GcStats gc_stats(const GcHeap *gc);

/* ── Symbol table ──────────────────────────────────────────────────────────
 *
 * Interns symbols so equal names share the same heap address (identity-based
 * equality). Backed by — and borrows — a GC. */
typedef struct GcSymbolTable GcSymbolTable;

GcSymbolTable *gc_symbol_table_new(GcHeap *gc);
void gc_symbol_table_free(GcSymbolTable *table);
/* Intern `name`, returning its (possibly newly allocated) address. */
size_t gc_symbol_table_intern(GcSymbolTable *table, const char *name);
/* Look up a live interned symbol; writes its address to *out_addr and returns
 * 1, or returns 0 if absent/dead. */
int gc_symbol_table_lookup(const GcSymbolTable *table, const char *name,
                           size_t *out_addr);
/* Number of currently-alive interned symbols. */
size_t gc_symbol_table_count(const GcSymbolTable *table);
/* Is `name` a currently-alive interned symbol? */
int gc_symbol_table_contains(const GcSymbolTable *table, const char *name);

#ifdef __cplusplus
}
#endif

#endif /* GARBAGE_COLLECTOR_H */
