/* Tests for the C B+ tree, using the iso_test.h harness. Mirrors the Rust
 * crate's checks: bulk insert → full leaf-chain scan (sorted) → validity →
 * search → range scan → delete → validity, across several degrees. */
#include "iso_test.h"

#include <stdlib.h>

#include "b_plus_tree.h"

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

/* Insert 0..n-1 as a coprime-stride permutation, then verify the leaf-chain
 * full scan is sorted and complete, validity, search, extremes, a range scan,
 * and deletion of even keys — for degree t. */
static void torture(size_t t, long n) {
    bpt *tree = bpt_new(t);
    long *kbuf = (long *)malloc((size_t)n * sizeof(long));
    long *vbuf = (long *)malloc((size_t)n * sizeof(long));
    collector col;
    long i;
    long mn = 0, mx = 0;
    int ok_sorted = 1, ok_search = 1;

    ISO_CHECK(tree != NULL && kbuf != NULL && vbuf != NULL);

    for (i = 0; i < n; i++) {
        long key = (i * 617 + 3) % n;
        ISO_CHECK(bpt_insert(tree, key, key * 10));
    }
    ISO_CHECK_EQ_UINT(bpt_len(tree), (size_t)n);
    ISO_CHECK(bpt_is_valid(tree));

    for (i = 0; i < n; i++) {
        long got = -1;
        if (!bpt_search(tree, i, &got) || got != i * 10) {
            ok_search = 0;
        }
    }
    ISO_CHECK_MSG(ok_search, "every inserted key must be found");
    ISO_CHECK(!bpt_contains(tree, n));

    ISO_CHECK(bpt_min_key(tree, &mn) && mn == 0);
    ISO_CHECK(bpt_max_key(tree, &mx) && mx == n - 1);

    /* Full scan walks the leaf chain and must yield 0..n-1 in order. */
    col.keys = kbuf;
    col.values = vbuf;
    col.count = 0;
    col.cap = (size_t)n;
    bpt_full_scan(tree, collect_cb, &col);
    ISO_CHECK_EQ_UINT(col.count, (size_t)n);
    for (i = 0; i < n; i++) {
        if (kbuf[i] != i || vbuf[i] != i * 10) {
            ok_sorted = 0;
        }
    }
    ISO_CHECK_MSG(ok_sorted, "full scan must be sorted and complete");

    /* Range scan [n/4, n/2] over the leaf chain. */
    col.count = 0;
    bpt_range_scan(tree, n / 4, n / 2, collect_cb, &col);
    ISO_CHECK_EQ_UINT(col.count, (size_t)(n / 2 - n / 4 + 1));
    ISO_CHECK(kbuf[0] == n / 4);

    /* Delete even keys; validity holds and the odds remain, chain still sorted. */
    for (i = 0; i < n; i += 2) {
        ISO_CHECK(bpt_delete(tree, i));
    }
    ISO_CHECK(bpt_is_valid(tree));
    ISO_CHECK_EQ_UINT(bpt_len(tree), (size_t)(n - (n + 1) / 2));
    {
        int ok = 1;
        for (i = 0; i < n; i++) {
            if (bpt_contains(tree, i) != (i % 2 != 0)) {
                ok = 0;
            }
        }
        ISO_CHECK_MSG(ok, "after deleting evens, exactly the odds remain");
    }
    ISO_CHECK(!bpt_delete(tree, 0)); /* already gone */

    free(kbuf);
    free(vbuf);
    bpt_free(tree);
}

int main(void) {
    /* Small hand-checkable example from the crate docs. */
    {
        bpt *tree = bpt_new(2);
        long v = 0;
        long kb[8], vb[8];
        collector col;
        ISO_CHECK(tree != NULL);
        ISO_CHECK(bpt_is_empty(tree));
        ISO_CHECK(bpt_insert(tree, 10, 100));
        ISO_CHECK(bpt_insert(tree, 5, 50));
        ISO_CHECK(bpt_insert(tree, 20, 200));
        ISO_CHECK(bpt_search(tree, 10, &v) && v == 100);
        /* range_scan(5, 15) → keys [5, 10]. */
        col.keys = kb;
        col.values = vb;
        col.count = 0;
        col.cap = 8;
        bpt_range_scan(tree, 5, 15, collect_cb, &col);
        ISO_CHECK_EQ_UINT(col.count, 2);
        ISO_CHECK(kb[0] == 5 && kb[1] == 10);
        ISO_CHECK(bpt_is_valid(tree));
        /* Overwrite keeps size. */
        ISO_CHECK(bpt_insert(tree, 10, 999));
        ISO_CHECK_EQ_UINT(bpt_len(tree), 3);
        ISO_CHECK(bpt_search(tree, 10, &v) && v == 999);
        bpt_free(tree);
    }

    /* Empty tree. */
    {
        bpt *tree = bpt_new(3);
        long v = 0;
        ISO_CHECK(tree != NULL);
        ISO_CHECK(!bpt_min_key(tree, &v));
        ISO_CHECK(!bpt_max_key(tree, &v));
        ISO_CHECK(!bpt_delete(tree, 1));
        ISO_CHECK(bpt_is_valid(tree));
        ISO_CHECK_EQ_UINT(bpt_height(tree), 0);
        bpt_free(tree);
    }

    /* Torture at several degrees (leaf/internal splits, borrows, merges). */
    torture(2, 1000);
    torture(3, 1500);
    torture(6, 2000);

    return ISO_TEST_RESULT();
}
