/* Tests for the C binary-search-tree, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include "binary_search_tree.h"

/* Insert a sequence of values into a fresh tree, returning the final tree.
 * Each bst_insert returns a NEW tree; we free the intermediate ones. */
static BST *build_from(const int *vals, size_t n) {
    BST *t = bst_empty();
    size_t i;
    for (i = 0; i < n; i++) {
        BST *next = bst_insert(t, vals[i]);
        bst_free(t);
        t = next;
    }
    return t;
}

int main(void) {
    /* --- insert / search / order statistics ---------------------------- */
    {
        int vals[] = {8, 3, 10, 1, 6, 14, 4, 7};
        BST *t = build_from(vals, 8);
        const BSTNode *node;
        int out;

        ISO_CHECK_EQ_UINT(bst_size(t), 8u);
        ISO_CHECK(bst_contains(t, 4));
        ISO_CHECK(!bst_contains(t, 99));

        node = bst_search(t, 4);
        ISO_CHECK(node != NULL && node->value == 4);
        ISO_CHECK(bst_search(t, 99) == NULL);

        ISO_CHECK(bst_min_value(t, &out) && out == 1);
        ISO_CHECK(bst_max_value(t, &out) && out == 14);

        /* rank(6) = number of values strictly less than 6 = {1,3,4} = 3. */
        ISO_CHECK_EQ_UINT(bst_rank(t, 6), 3u);
        /* kth_smallest(4) (1-based): sorted 1,3,4,6,... -> 4th is 6. */
        ISO_CHECK(bst_kth_smallest(t, 4, &out) && out == 6);
        /* kth_smallest(1) -> min. */
        ISO_CHECK(bst_kth_smallest(t, 1, &out) && out == 1);
        /* kth_smallest(8) -> max. */
        ISO_CHECK(bst_kth_smallest(t, 8, &out) && out == 14);
        /* out of range. */
        ISO_CHECK(!bst_kth_smallest(t, 0, &out));
        ISO_CHECK(!bst_kth_smallest(t, 9, &out));

        /* predecessor / successor. */
        ISO_CHECK(bst_predecessor(t, 6, &out) && out == 4);
        ISO_CHECK(bst_successor(t, 6, &out) && out == 7);
        ISO_CHECK(!bst_predecessor(t, 1, &out)); /* none below min */
        ISO_CHECK(!bst_successor(t, 14, &out));  /* none above max */

        ISO_CHECK(bst_is_valid(t));
        bst_free(t);
    }

    /* --- persistence: insert returns a new tree, leaves original ------- */
    {
        int vals[] = {5, 3, 8};
        BST *t = build_from(vals, 3);
        BST *t2 = bst_insert(t, 1);
        ISO_CHECK_EQ_UINT(bst_size(t), 3u);  /* original untouched */
        ISO_CHECK_EQ_UINT(bst_size(t2), 4u);
        ISO_CHECK(!bst_contains(t, 1));
        ISO_CHECK(bst_contains(t2, 1));
        /* duplicate insert is a no-op (set semantics). */
        {
            BST *t3 = bst_insert(t2, 8);
            ISO_CHECK_EQ_UINT(bst_size(t3), 4u);
            bst_free(t3);
        }
        bst_free(t2);
        bst_free(t);
    }

    /* --- delete (all three node shapes) -------------------------------- */
    {
        int vals[] = {8, 3, 10, 1, 6, 14, 4, 7};
        BST *t = build_from(vals, 8);

        /* delete a two-child node (3 has children 1 and 6). */
        BST *d = bst_delete(t, 3);
        ISO_CHECK_EQ_UINT(bst_size(t), 8u); /* original untouched */
        ISO_CHECK(!bst_contains(d, 3));
        ISO_CHECK_EQ_UINT(bst_size(d), 7u);
        ISO_CHECK(bst_is_valid(d));

        /* delete a leaf (7). */
        {
            BST *d2 = bst_delete(d, 7);
            ISO_CHECK(!bst_contains(d2, 7));
            ISO_CHECK_EQ_UINT(bst_size(d2), 6u);
            ISO_CHECK(bst_is_valid(d2));
            bst_free(d2);
        }
        /* delete a one-child node (10 has only child 14). */
        {
            BST *d2 = bst_delete(d, 10);
            ISO_CHECK(!bst_contains(d2, 10));
            ISO_CHECK(bst_contains(d2, 14));
            ISO_CHECK(bst_is_valid(d2));
            bst_free(d2);
        }
        /* delete a missing value is a no-op. */
        {
            BST *d2 = bst_delete(d, 999);
            ISO_CHECK_EQ_UINT(bst_size(d2), 7u);
            ISO_CHECK(bst_is_valid(d2));
            bst_free(d2);
        }
        bst_free(d);
        bst_free(t);
    }

    /* --- from_sorted_array: balanced, in-order round trip -------------- */
    {
        int sorted[] = {1, 2, 3, 4, 5, 6, 7};
        BST *t = bst_from_sorted_array(sorted, 7);
        int buf[7];
        size_t got, i;

        ISO_CHECK_EQ_UINT(bst_size(t), 7u);
        ISO_CHECK(bst_is_valid(t));
        /* 7 nodes balanced -> height 2 (levels 0,1,2). */
        ISO_CHECK(bst_height(t) <= 2);

        got = bst_to_sorted_array(t, buf, 7);
        ISO_CHECK_EQ_UINT(got, 7u);
        for (i = 0; i < 7; i++) {
            ISO_CHECK_EQ_INT(buf[i], (int)(i + 1));
        }
        bst_free(t);
    }

    /* --- empty tree edge cases ----------------------------------------- */
    {
        BST *t = bst_empty();
        int out, buf[4];
        ISO_CHECK_EQ_UINT(bst_size(t), 0u);
        ISO_CHECK_EQ_INT((int)bst_height(t), -1);
        ISO_CHECK(bst_is_valid(t));
        ISO_CHECK(!bst_contains(t, 0));
        ISO_CHECK(!bst_min_value(t, &out));
        ISO_CHECK(!bst_max_value(t, &out));
        ISO_CHECK(!bst_kth_smallest(t, 1, &out));
        ISO_CHECK_EQ_UINT(bst_rank(t, 5), 0u);
        ISO_CHECK_EQ_UINT(bst_to_sorted_array(t, buf, 4), 0u);
        bst_free(t);
    }

    return ISO_TEST_RESULT();
}
