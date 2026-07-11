/*
 * fenwick_tree.c — implementation of the double-valued Fenwick tree. Ported
 * from the Rust `fenwick-tree` crate; the index walks, 1-based layout, and
 * find_kth binary-lifting all match it.
 */
#include "fenwick_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */

/* lowbit(i) — the value of the lowest set bit of i (i & -i). We negate in
 * unsigned space to avoid signed-overflow UB. */
static size_t lowbit(size_t i) {
    return i & (~i + 1u);
}

/* highest_power_of_two_at_most(n) — largest 2^k <= n (0 when n == 0). */
static size_t highest_power_of_two_at_most(size_t n) {
    size_t p = 1;
    if (n == 0) {
        return 0;
    }
    while (p <= n / 2) {
        p *= 2;
    }
    return p;
}

fenwick_status fenwick_init(fenwick_tree *t, size_t n) {
    /* Reject n == SIZE_MAX up front: n + 1 would wrap to 0, calloc(0, ...) could
     * return a minimal buffer, and later 1..=n accesses would run wildly out of
     * bounds. (calloc guards the multiply, but not our n + 1 addition.) */
    if (n == SIZE_MAX) {
        t->bit = NULL;
        t->n = 0;
        return FENWICK_ALLOC_FAILED;
    }
    /* calloc zeroes bit[0..n]; the +1 is the unused 1-based slot 0. */
    t->bit = (double *)calloc(n + 1, sizeof(double));
    if (t->bit == NULL) {
        t->n = 0;
        return FENWICK_ALLOC_FAILED;
    }
    t->n = n;
    return FENWICK_OK;
}

fenwick_status fenwick_init_from_slice(fenwick_tree *t, const double *values,
                                       size_t count) {
    size_t index;
    fenwick_status st = fenwick_init(t, count);
    if (st != FENWICK_OK) {
        return st;
    }
    /* Build in O(n): each slot absorbs its element, then pushes its running
     * total up to its parent (index + lowbit(index)). Same as the crate. */
    for (index = 1; index <= t->n; index++) {
        size_t parent = index + lowbit(index);
        t->bit[index] += values[index - 1];
        if (parent <= t->n) {
            t->bit[parent] += t->bit[index];
        }
    }
    return FENWICK_OK;
}

void fenwick_free(fenwick_tree *t) {
    free(t->bit);
    t->bit = NULL;
    t->n = 0;
}

fenwick_status fenwick_update(fenwick_tree *t, size_t index, double delta) {
    size_t current;
    if (index < 1 || index > t->n) {
        return FENWICK_INDEX_OUT_OF_RANGE;
    }
    for (current = index; current <= t->n; current += lowbit(current)) {
        t->bit[current] += delta;
    }
    return FENWICK_OK;
}

fenwick_status fenwick_prefix_sum(const fenwick_tree *t, size_t index,
                                  double *out) {
    double total = 0.0;
    size_t current;
    if (index > t->n) { /* index 0 is allowed: the empty prefix */
        return FENWICK_INDEX_OUT_OF_RANGE;
    }
    for (current = index; current > 0; current -= lowbit(current)) {
        total += t->bit[current];
    }
    *out = total;
    return FENWICK_OK;
}

fenwick_status fenwick_range_sum(const fenwick_tree *t, size_t left,
                                 size_t right, double *out) {
    if (left > right) {
        return FENWICK_INVALID_RANGE;
    }
    if (left < 1 || left > t->n || right < 1 || right > t->n) {
        return FENWICK_INDEX_OUT_OF_RANGE;
    }
    if (left == 1) {
        return fenwick_prefix_sum(t, right, out);
    }
    {
        double hi, lo;
        fenwick_status st = fenwick_prefix_sum(t, right, &hi);
        if (st != FENWICK_OK) {
            return st;
        }
        st = fenwick_prefix_sum(t, left - 1, &lo);
        if (st != FENWICK_OK) {
            return st;
        }
        *out = hi - lo;
        return FENWICK_OK;
    }
}

fenwick_status fenwick_point_query(const fenwick_tree *t, size_t index,
                                   double *out) {
    if (index < 1 || index > t->n) {
        return FENWICK_INDEX_OUT_OF_RANGE;
    }
    return fenwick_range_sum(t, index, index, out);
}

fenwick_status fenwick_find_kth(const fenwick_tree *t, double target,
                                size_t *out) {
    size_t index = 0;
    size_t step;
    double total;
    fenwick_status st;

    if (t->n == 0) {
        return FENWICK_EMPTY_TREE;
    }
    if (target <= 0.0) {
        return FENWICK_NON_POSITIVE_TARGET;
    }
    st = fenwick_prefix_sum(t, t->n, &total);
    if (st != FENWICK_OK) {
        return st;
    }
    if (target > total) {
        return FENWICK_TARGET_EXCEEDS_TOTAL;
    }
    /* Binary lifting: try to extend `index` by the largest power of two while
     * the covered prefix sum stays below target. */
    for (step = highest_power_of_two_at_most(t->n); step > 0; step >>= 1) {
        size_t next = index + step;
        if (next <= t->n && t->bit[next] < target) {
            index = next;
            target -= t->bit[index];
        }
    }
    *out = index + 1;
    return FENWICK_OK;
}

size_t fenwick_len(const fenwick_tree *t) {
    return t->n;
}

int fenwick_is_empty(const fenwick_tree *t) {
    return t->n == 0 ? 1 : 0;
}
