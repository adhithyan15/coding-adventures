/*
 * heap.h — a binary heap (priority queue) of ints, in pure ISO C17. A faithful
 * port of the Rust `heap` crate (MinHeap / MaxHeap).
 * ===========================================================================
 *
 * A binary heap keeps its elements in an array laid out as a complete binary
 * tree: the children of index i live at 2i+1 and 2i+2, the parent at (i-1)/2.
 * A single "priority" rule decides what floats to the top:
 *   • a MIN heap keeps the SMALLEST element at the root (index 0)
 *   • a MAX heap keeps the LARGEST
 * push and pop each restore the heap order in O(log n) by sifting an element up
 * or down against that rule.
 *
 * The heap owns a growable array, so pair heap_init with heap_free. Fallible
 * operations return 1 on success and 0 on failure (empty pop/peek, or an
 * allocation failure on push).
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HEAP_H
#define HEAP_H

#include <stddef.h> /* size_t */

/* Which end of the ordering sits at the root. */
typedef enum { HEAP_MIN = 0, HEAP_MAX = 1 } heap_order;

/* The heap. Treat the fields as opaque; use the functions below. */
typedef struct {
    int *data;
    size_t len;
    size_t cap;
    heap_order order;
} heap;

/* heap_init — start an empty heap with the given ordering. Allocates nothing
 * up front (the first push grows the buffer); always returns 1. */
int heap_init(heap *h, heap_order order);

/* heap_free — release storage. Safe on a zeroed struct; idempotent. */
void heap_free(heap *h);

/* heap_push — insert `value`. Returns 1, or 0 if growing the buffer failed. */
int heap_push(heap *h, int value);

/* heap_pop — remove and return the root (min or max) through *out.
 * Returns 1, or 0 if the heap is empty (*out untouched). */
int heap_pop(heap *h, int *out);

/* heap_peek — read the root without removing it, through *out.
 * Returns 1, or 0 if empty (*out untouched). */
int heap_peek(const heap *h, int *out);

/* heap_len — number of elements. heap_is_empty — 1 if empty else 0. */
size_t heap_len(const heap *h);
int heap_is_empty(const heap *h);

/* heap_sort — sort `arr[0..n]` in ASCENDING order in place, using a heap
 * internally (mirrors the crate's heap_sort, which drains a MinHeap). Returns 1
 * on success, or 0 if a temporary allocation failed (in which case `arr` is
 * left unchanged). */
int heap_sort(int *arr, size_t n);

#endif /* HEAP_H */
