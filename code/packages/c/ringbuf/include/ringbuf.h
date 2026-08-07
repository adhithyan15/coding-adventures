/*
 * ringbuf.h — a fixed-capacity ring (circular) buffer of ints, in pure ISO C17.
 * ===========================================================================
 *
 * A ring buffer stores up to a fixed number of elements in a plain array and
 * treats that array as if its ends were joined into a circle. Two numbers track
 * the live region:
 *
 *     head  → index of the oldest element (where the next pop() reads)
 *     count → how many elements are currently stored
 *
 * The next push() writes at (head + count) mod cap. When an index runs off the
 * end of the array it wraps back to 0 — that wraparound is the whole trick, and
 * it makes push/pop O(1) with no shifting of elements.
 *
 *   capacity = 4, after push(10) push(20) push(30):
 *
 *       index:   0    1    2    3
 *              [ 10 | 20 | 30 |    ]
 *                ^head          ^ next write = (0 + 3) % 4 = 3
 *
 *   then pop() → returns 10, head advances to 1, count = 2:
 *
 *              [ .. | 20 | 30 |    ]
 *                     ^head
 *
 * The buffer does NOT own memory: the caller supplies the backing array, so it
 * works with stack, static, or heap storage and pulls in no allocator. That
 * keeps it pure ISO C with zero dependencies.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef RINGBUF_H
#define RINGBUF_H

#include <stddef.h> /* size_t */

/* The ring buffer control block. Treat the fields as read-only from outside;
 * mutate them only through the functions below. */
typedef struct {
    int *buf;     /* caller-owned backing array of `cap` ints */
    size_t cap;   /* capacity — the length of `buf` */
    size_t head;  /* index of the oldest element (next to pop) */
    size_t count; /* number of elements currently stored */
} ringbuf;

/* ringbuf_init — bind a ring buffer to a caller-owned backing array.
 * `backing` must point to at least `cap` ints and must outlive the ring buffer.
 * A `cap` of 0 yields a buffer that is permanently both empty and full. */
void ringbuf_init(ringbuf *r, int *backing, size_t cap);

/* ringbuf_capacity / ringbuf_count — total slots and currently-used slots. */
size_t ringbuf_capacity(const ringbuf *r);
size_t ringbuf_count(const ringbuf *r);

/* ringbuf_is_empty / ringbuf_is_full — 1 (true) or 0 (false). */
int ringbuf_is_empty(const ringbuf *r);
int ringbuf_is_full(const ringbuf *r);

/* ringbuf_push — append `value` at the tail.
 * Returns 1 on success, or 0 if the buffer is full (value not stored). */
int ringbuf_push(ringbuf *r, int value);

/* ringbuf_pop — remove the oldest element and store it through `out`.
 * Returns 1 on success, or 0 if the buffer is empty (`*out` untouched). */
int ringbuf_pop(ringbuf *r, int *out);

/* ringbuf_peek — read the oldest element WITHOUT removing it.
 * Returns 1 on success, or 0 if the buffer is empty (`*out` untouched). */
int ringbuf_peek(const ringbuf *r, int *out);

/* ringbuf_clear — drop all elements. The backing array is left as-is. */
void ringbuf_clear(ringbuf *r);

#endif /* RINGBUF_H */
