/*
 * heap.c — implementation of the int binary heap. Ported from the Rust `heap`
 * crate; the sift-up/sift-down logic and the "higher priority" rule match it.
 */
#include "heap.h"

#include <stdlib.h> /* malloc, realloc, free */

/* higher_priority(h, a, b) — true if `a` should sit above `b` in this heap.
 * MIN: smaller wins; MAX: larger wins. */
static int higher_priority(const heap *h, int a, int b) {
    return h->order == HEAP_MIN ? (a < b) : (a > b);
}

static void swap_int(int *x, int *y) {
    int tmp = *x;
    *x = *y;
    *y = tmp;
}

/* Restore heap order by moving the element at `index` toward the root. */
static void sift_up(heap *h, size_t index) {
    while (index > 0) {
        size_t parent = (index - 1) / 2;
        if (higher_priority(h, h->data[index], h->data[parent])) {
            swap_int(&h->data[index], &h->data[parent]);
            index = parent;
        } else {
            break;
        }
    }
}

/* Restore heap order by moving the element at `index` toward the leaves. */
static void sift_down(heap *h, size_t index) {
    for (;;) {
        size_t left = 2 * index + 1;
        size_t right = 2 * index + 2;
        size_t best = index;
        if (left < h->len && higher_priority(h, h->data[left], h->data[best])) {
            best = left;
        }
        if (right < h->len && higher_priority(h, h->data[right], h->data[best])) {
            best = right;
        }
        if (best == index) {
            return;
        }
        swap_int(&h->data[index], &h->data[best]);
        index = best;
    }
}

int heap_init(heap *h, heap_order order) {
    h->data = NULL;
    h->len = 0;
    h->cap = 0;
    h->order = order;
    return 1;
}

void heap_free(heap *h) {
    free(h->data);
    h->data = NULL;
    h->len = 0;
    h->cap = 0;
}

/* Ensure room for at least one more element, doubling the buffer as needed. */
static int ensure_capacity(heap *h) {
    size_t new_cap;
    int *grown;
    if (h->len < h->cap) {
        return 1;
    }
    new_cap = h->cap == 0 ? 4 : h->cap * 2;
    grown = (int *)realloc(h->data, new_cap * sizeof(int));
    if (grown == NULL) {
        return 0;
    }
    h->data = grown;
    h->cap = new_cap;
    return 1;
}

int heap_push(heap *h, int value) {
    if (!ensure_capacity(h)) {
        return 0;
    }
    h->data[h->len] = value;
    h->len++;
    sift_up(h, h->len - 1);
    return 1;
}

int heap_pop(heap *h, int *out) {
    if (h->len == 0) {
        return 0;
    }
    *out = h->data[0];
    h->len--;
    if (h->len > 0) {
        /* Move the last element to the root and sift it down. */
        h->data[0] = h->data[h->len];
        sift_down(h, 0);
    }
    return 1;
}

int heap_peek(const heap *h, int *out) {
    if (h->len == 0) {
        return 0;
    }
    *out = h->data[0];
    return 1;
}

size_t heap_len(const heap *h) {
    return h->len;
}

int heap_is_empty(const heap *h) {
    return h->len == 0 ? 1 : 0;
}

int heap_sort(int *arr, size_t n) {
    heap h;
    size_t i;
    heap_init(&h, HEAP_MIN);
    for (i = 0; i < n; i++) {
        if (!heap_push(&h, arr[i])) {
            heap_free(&h);
            return 0; /* leave arr unchanged on allocation failure */
        }
    }
    /* Draining a min-heap yields ascending order. */
    for (i = 0; i < n; i++) {
        int value;
        heap_pop(&h, &value);
        arr[i] = value;
    }
    heap_free(&h);
    return 1;
}
