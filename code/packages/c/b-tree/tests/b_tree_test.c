/* Tests for the C B-tree, using the iso_test.h harness. Mirrors the Rust crate's
 * strong checks: bulk insert → sorted in-order → structural validity → delete →
 * range query, across several minimum degrees. */
#include "iso_test.h"

#include <stdlib.h>

#include "b_tree.h"

/* Collector for the ordered traversals. */
typedef struct {
    long *keys;
    long *values;
    size_t count;
    size_t cap;
} collector;

static void collect_cb(long k, long v, void *u) {
    collector *c = (collector *)u;
    if (c->count < c->cap) {
        c->keys[c->count] = k;
        c->values[c->count] = v;
    }
    c->count++;
}

/* Insert 0..N-1 (as a coprime-stride permutation, so inserts arrive out of
 * order), then verify sorted in-order, validity, search, min/max, deletion of
 * even keys, and a range query — for the given minimum degree t. */
static void torture(size_t t, long n) {
    btree *tree = btree_new(t);
    long *kbuf = (long *)malloc((size_t)n * sizeof(long));
    long *vbuf = (long *)malloc((size_t)n * sizeof(long));
    collector col;
    long i;
    long mn = 0, mx = 0;
    int ok_sorted = 1, ok_search = 1;

    ISO_CHECK(tree != NULL && kbuf != NULL && vbuf != NULL);

    /* 617 is coprime with our n values, so (i*617+3)%n is a permutation. */
    for (i = 0; i < n; i++) {
        long key = (i * 617 + 3) % n;
        ISO_CHECK(btree_insert(tree, key, key * 10));
    }
    ISO_CHECK_EQ_UINT(btree_len(tree), (size_t)n);
    ISO_CHECK(btree_is_valid(tree));

    /* Every key is present with the right value. */
    for (i = 0; i < n; i++) {
        long got = -1;
        if (!btree_search(tree, i, &got) || got != i * 10) {
            ok_search = 0;
        }
    }
    ISO_CHECK_MSG(ok_search, "every inserted key must be found");
    ISO_CHECK(!btree_contains(tree, n));   /* absent */
    ISO_CHECK(!btree_contains(tree, -1));

    ISO_CHECK(btree_min_key(tree, &mn) && mn == 0);
    ISO_CHECK(btree_max_key(tree, &mx) && mx == n - 1);

    /* In-order traversal yields 0,1,2,...,n-1. */
    col.keys = kbuf;
    col.values = vbuf;
    col.count = 0;
    col.cap = (size_t)n;
    btree_inorder(tree, collect_cb, &col);
    ISO_CHECK_EQ_UINT(col.count, (size_t)n);
    for (i = 0; i < n; i++) {
        if (kbuf[i] != i || vbuf[i] != i * 10) {
            ok_sorted = 0;
        }
    }
    ISO_CHECK_MSG(ok_sorted, "in-order traversal must be sorted and complete");

    /* Range query [n/4, n/2] returns exactly that inclusive band. */
    col.count = 0;
    btree_range_query(tree, n / 4, n / 2, collect_cb, &col);
    ISO_CHECK_EQ_UINT(col.count, (size_t)(n / 2 - n / 4 + 1));
    ISO_CHECK(kbuf[0] == n / 4);

    /* Delete all even keys; validity holds throughout, odds remain. */
    for (i = 0; i < n; i += 2) {
        ISO_CHECK(btree_delete(tree, i));
    }
    ISO_CHECK(btree_is_valid(tree));
    ISO_CHECK_EQ_UINT(btree_len(tree), (size_t)(n - (n + 1) / 2));
    {
        int ok = 1;
        for (i = 0; i < n; i++) {
            int present = btree_contains(tree, i);
            int expect = (i % 2 != 0);
            if (present != expect) {
                ok = 0;
            }
        }
        ISO_CHECK_MSG(ok, "after deleting evens, exactly the odds remain");
    }
    /* Deleting an already-absent key returns 0. */
    ISO_CHECK(!btree_delete(tree, 0));

    free(kbuf);
    free(vbuf);
    btree_free(tree);
}

int main(void) {
    /* Small, hand-checkable example (t = 2, a 2-3-4 tree). */
    {
        btree *tree = btree_new(2);
        long v = 0;
        ISO_CHECK(tree != NULL);
        ISO_CHECK(btree_is_empty(tree));
        ISO_CHECK(btree_insert(tree, 10, 100));
        ISO_CHECK(btree_insert(tree, 20, 200));
        ISO_CHECK(btree_insert(tree, 5, 50));
        ISO_CHECK_EQ_UINT(btree_len(tree), 3);
        ISO_CHECK(btree_search(tree, 10, &v) && v == 100);
        ISO_CHECK(!btree_search(tree, 99, &v));
        ISO_CHECK(btree_min_key(tree, &v) && v == 5);
        ISO_CHECK(btree_max_key(tree, &v) && v == 20);
        ISO_CHECK(btree_is_valid(tree));
        /* Overwrite keeps the size. */
        ISO_CHECK(btree_insert(tree, 10, 999));
        ISO_CHECK_EQ_UINT(btree_len(tree), 3);
        ISO_CHECK(btree_search(tree, 10, &v) && v == 999);
        /* Delete down to empty. */
        ISO_CHECK(btree_delete(tree, 10));
        ISO_CHECK(btree_delete(tree, 20));
        ISO_CHECK(btree_delete(tree, 5));
        ISO_CHECK(btree_is_empty(tree));
        ISO_CHECK_EQ_UINT(btree_height(tree), 0);
        btree_free(tree);
    }

    /* Empty tree behaviour. */
    {
        btree *tree = btree_new(3);
        long v = 0;
        ISO_CHECK(tree != NULL);
        ISO_CHECK(!btree_min_key(tree, &v));
        ISO_CHECK(!btree_max_key(tree, &v));
        ISO_CHECK(!btree_delete(tree, 1));
        ISO_CHECK(btree_is_valid(tree));
        ISO_CHECK_EQ_UINT(btree_len(tree), 0);
        btree_free(tree);
    }

    /* Torture tests at several degrees (splits, merges, borrows all exercised). */
    torture(2, 1000);
    torture(3, 1500);
    torture(7, 2000);

    return ISO_TEST_RESULT();
}
