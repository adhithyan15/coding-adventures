/*
 * ringbuf.c — implementation of the fixed-capacity int ring buffer.
 *
 * The only subtlety is the modular index arithmetic. We deliberately keep an
 * explicit `count` rather than a second `tail` index: with head+tail you cannot
 * tell "completely full" from "completely empty" (both look like head == tail)
 * without wasting a slot or keeping an extra flag. head+count sidesteps that —
 * full is simply count == cap.
 */
#include "ringbuf.h"

void ringbuf_init(ringbuf *r, int *backing, size_t cap) {
    r->buf = backing;
    r->cap = cap;
    r->head = 0;
    r->count = 0;
}

size_t ringbuf_capacity(const ringbuf *r) {
    return r->cap;
}

size_t ringbuf_count(const ringbuf *r) {
    return r->count;
}

int ringbuf_is_empty(const ringbuf *r) {
    return r->count == 0 ? 1 : 0;
}

int ringbuf_is_full(const ringbuf *r) {
    return r->count == r->cap ? 1 : 0;
}

int ringbuf_push(ringbuf *r, int value) {
    size_t tail;
    if (r->count == r->cap) {
        return 0; /* full */
    }
    /* The write position is `count` slots ahead of head, wrapping around. */
    tail = r->head + r->count;
    if (tail >= r->cap) {
        tail -= r->cap;
    }
    r->buf[tail] = value;
    r->count++;
    return 1;
}

int ringbuf_pop(ringbuf *r, int *out) {
    if (r->count == 0) {
        return 0; /* empty */
    }
    *out = r->buf[r->head];
    /* Advance head, wrapping back to 0 at the end of the array. */
    r->head++;
    if (r->head >= r->cap) {
        r->head = 0;
    }
    r->count--;
    return 1;
}

int ringbuf_peek(const ringbuf *r, int *out) {
    if (r->count == 0) {
        return 0; /* empty */
    }
    *out = r->buf[r->head];
    return 1;
}

void ringbuf_clear(ringbuf *r) {
    r->head = 0;
    r->count = 0;
}
