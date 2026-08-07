/* Tests for the C avl-tree, using the iso_test.h harness. Mirrors the Rust
 * crate's unit tests (rotations rebalance; search + order statistics) and adds
 * coverage for delete, predecessor/successor, and persistence. */
#include "iso_test.h"

#include <stdlib.h> /* NULL */

#include "avl_tree.h"

/* Build a tree by inserting `values` in order; each insert returns a fresh
 * tree, and we free the intermediate. Returns the final tree (caller frees). */
static AVLTree *build(const int *values, size_t n) {
    AVLTree *t = avl_empty();
    size_t i;
    for (i = 0; i < n; i++) {
        AVLTree *next = avl_insert(t, values[i]);
        avl_free(t);
        t = next;
    }
    return t;
}

int main(void) {
    /* Rust test: rotations_rebalance_the_tree.
     * Inserting 30,20,10 (LL case) and 10,20,30 (RR case) both rebalance so 20
     * becomes the root. */
    {
        int desc[] = {30, 20, 10};
        int asc[] = {10, 20, 30};
        AVLTree *t1 = build(desc, 3);
        AVLTree *t2 = build(asc, 3);
        ISO_CHECK_EQ_INT(avl_search(t1, 20)->value, 20); /* root holds 20 */
        ISO_CHECK_EQ_INT(t1->root->value, 20);
        ISO_CHECK(avl_is_valid_avl(t1));
        ISO_CHECK_EQ_INT(t2->root->value, 20);
        ISO_CHECK(avl_is_valid_avl(t2));
        avl_free(t1);
        avl_free(t2);
    }

    /* Rust test: search_and_order_statistics_work.
     * Insert 8,3,10,1,6,14,4,7. */
    {
        int vs[] = {8, 3, 10, 1, 6, 14, 4, 7};
        int out = 0;
        int sorted[8];
        int expected[8] = {1, 3, 4, 6, 7, 8, 10, 14};
        size_t written, i;
        AVLTree *t = build(vs, 8);

        ISO_CHECK(avl_contains(t, 6));
        ISO_CHECK(!avl_contains(t, 99));
        ISO_CHECK(avl_min_value(t, &out) && out == 1);
        ISO_CHECK(avl_max_value(t, &out) && out == 14);
        ISO_CHECK_EQ_UINT(avl_rank(t, 6), 3u);
        ISO_CHECK(avl_kth_smallest(t, 4, &out) && out == 6);
        ISO_CHECK_EQ_UINT(avl_size(t), 8u);
        ISO_CHECK(avl_is_valid_bst(t));
        ISO_CHECK(avl_is_valid_avl(t));

        written = avl_to_sorted_array(t, sorted, 8);
        ISO_CHECK_EQ_UINT(written, 8u);
        for (i = 0; i < 8; i++) {
            ISO_CHECK_EQ_INT(sorted[i], expected[i]);
        }
        avl_free(t);
    }

    /* Empty tree edge cases. */
    {
        AVLTree *e = avl_empty();
        int out = 42;
        ISO_CHECK_EQ_INT(avl_height(e), -1L);
        ISO_CHECK_EQ_UINT(avl_size(e), 0u);
        ISO_CHECK(!avl_min_value(e, &out) && out == 42);
        ISO_CHECK(!avl_kth_smallest(e, 1, &out));
        ISO_CHECK(avl_search(e, 5) == NULL);
        ISO_CHECK(avl_is_valid_avl(e));
        avl_free(e);
    }

    /* Predecessor / successor. */
    {
        int vs[] = {8, 3, 10, 1, 6, 14, 4, 7};
        int out = 0;
        AVLTree *t = build(vs, 8);
        ISO_CHECK(avl_predecessor(t, 6, &out) && out == 4);
        ISO_CHECK(avl_successor(t, 6, &out) && out == 7);
        ISO_CHECK(!avl_predecessor(t, 1, &out)); /* nothing below the min */
        ISO_CHECK(!avl_successor(t, 14, &out));   /* nothing above the max */
        ISO_CHECK(avl_predecessor(t, 5, &out) && out == 4); /* absent query */
        ISO_CHECK(avl_successor(t, 5, &out) && out == 6);
        avl_free(t);
    }

    /* Delete: leaf, one-child, and two-children (successor replacement) cases,
     * each keeping the AVL invariant. */
    {
        int vs[] = {50, 30, 70, 20, 40, 60, 80, 35, 45};
        int sorted[16];
        size_t written;
        int expected[8] = {20, 35, 40, 45, 50, 60, 70, 80};
        size_t i;
        AVLTree *t = build(vs, 9);
        AVLTree *d1 = avl_delete(t, 30); /* two children (20 + 40 subtree) */
        ISO_CHECK(!avl_contains(d1, 30));
        ISO_CHECK(avl_is_valid_avl(d1));
        ISO_CHECK_EQ_UINT(avl_size(d1), 8u);
        /* original tree untouched (persistence) */
        ISO_CHECK(avl_contains(t, 30));
        ISO_CHECK_EQ_UINT(avl_size(t), 9u);

        written = avl_to_sorted_array(d1, sorted, 16);
        ISO_CHECK_EQ_UINT(written, 8u);
        for (i = 0; i < 8; i++) {
            ISO_CHECK_EQ_INT(sorted[i], expected[i]);
        }

        /* Deleting an absent value yields an equal-size independent copy. */
        {
            AVLTree *d2 = avl_delete(t, 999);
            ISO_CHECK_EQ_UINT(avl_size(d2), 9u);
            ISO_CHECK(avl_is_valid_avl(d2));
            avl_free(d2);
        }
        avl_free(d1);
        avl_free(t);
    }

    /* Duplicate insert is a no-op on membership but still an independent copy. */
    {
        int vs[] = {5, 5, 5};
        AVLTree *t = build(vs, 3);
        ISO_CHECK_EQ_UINT(avl_size(t), 1u);
        ISO_CHECK(avl_is_valid_avl(t));
        avl_free(t);
    }

    /* Larger stress: insert 0..99, verify balance + order statistics, then
     * delete the evens and re-verify. */
    {
        int i;
        int out = 0;
        AVLTree *t = avl_empty();
        for (i = 0; i < 100; i++) {
            AVLTree *n = avl_insert(t, i);
            avl_free(t);
            t = n;
        }
        ISO_CHECK_EQ_UINT(avl_size(t), 100u);
        ISO_CHECK(avl_is_valid_avl(t));
        /* height of a 100-node AVL tree is well under 2*log2(100) ~ 13 */
        ISO_CHECK(avl_height(t) <= 8L);
        ISO_CHECK(avl_kth_smallest(t, 50, &out) && out == 49);
        ISO_CHECK_EQ_UINT(avl_rank(t, 75), 75u);

        for (i = 0; i < 100; i += 2) {
            AVLTree *n = avl_delete(t, i);
            avl_free(t);
            t = n;
        }
        ISO_CHECK_EQ_UINT(avl_size(t), 50u);
        ISO_CHECK(avl_is_valid_avl(t));
        ISO_CHECK(!avl_contains(t, 42));
        ISO_CHECK(avl_contains(t, 43));
        avl_free(t);
    }

    return ISO_TEST_RESULT();
}
