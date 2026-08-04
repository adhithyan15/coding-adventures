/* Tests for the C red-black-tree, using the iso_test.h harness. Mirrors the
 * Rust crate's unit tests and adds delete cases, neighbour queries,
 * persistence, and a stress that re-checks the LLRB invariant throughout. */
#include "iso_test.h"

#include <stdlib.h> /* NULL */

#include "red_black_tree.h"

/* Insert `values` in order, freeing intermediates; returns the final tree. */
static RBTree *build(const int *values, size_t n) {
    RBTree *t = rb_empty();
    size_t i;
    for (i = 0; i < n; i++) {
        RBTree *next = rb_insert(t, values[i]);
        rb_free(t);
        t = next;
    }
    return t;
}

int main(void) {
    /* Rust test: insert_search_and_delete_work. */
    {
        int vs[] = {8, 3, 10, 1, 6, 14, 4, 7};
        int out = 0;
        RBTree *t = build(vs, 8);
        RBTree *d;
        ISO_CHECK(rb_contains(t, 6));
        ISO_CHECK(rb_min_value(t, &out) && out == 1);
        ISO_CHECK(rb_max_value(t, &out) && out == 14);
        ISO_CHECK(rb_kth_smallest(t, 4, &out) && out == 6);
        ISO_CHECK(rb_is_valid_rb(t));
        ISO_CHECK(t->root->color == RB_BLACK); /* root is black */

        d = rb_delete(t, 3);
        ISO_CHECK(!rb_contains(d, 3));
        ISO_CHECK(rb_is_valid_rb(d));
        ISO_CHECK(rb_contains(t, 3)); /* original untouched (persistence) */
        ISO_CHECK_EQ_UINT(rb_size(t), 8u);
        ISO_CHECK_EQ_UINT(rb_size(d), 7u);
        rb_free(d);
        rb_free(t);
    }

    /* Rust test: black_height_and_sorted_output_work. */
    {
        int vs[] = {2, 1, 3};
        int sorted[3];
        int expected[3] = {1, 2, 3};
        size_t written, i;
        RBTree *t = build(vs, 3);
        ISO_CHECK(rb_black_height(t) >= 1u);
        written = rb_to_sorted_array(t, sorted, 3);
        ISO_CHECK_EQ_UINT(written, 3u);
        for (i = 0; i < 3; i++) {
            ISO_CHECK_EQ_INT(sorted[i], expected[i]);
        }
        rb_free(t);
    }

    /* Empty-tree edge cases. */
    {
        RBTree *e = rb_empty();
        int out = 42;
        ISO_CHECK_EQ_UINT(rb_size(e), 0u);
        ISO_CHECK_EQ_UINT(rb_black_height(e), 0u);
        ISO_CHECK(!rb_min_value(e, &out) && out == 42);
        ISO_CHECK(!rb_kth_smallest(e, 1, &out));
        ISO_CHECK(rb_search(e, 5) == NULL);
        ISO_CHECK(rb_is_valid_rb(e));
        rb_free(e);
    }

    /* Predecessor / successor. */
    {
        int vs[] = {8, 3, 10, 1, 6, 14, 4, 7};
        int out = 0;
        RBTree *t = build(vs, 8);
        ISO_CHECK(rb_predecessor(t, 6, &out) && out == 4);
        ISO_CHECK(rb_successor(t, 6, &out) && out == 7);
        ISO_CHECK(!rb_predecessor(t, 1, &out));
        ISO_CHECK(!rb_successor(t, 14, &out));
        ISO_CHECK(rb_predecessor(t, 5, &out) && out == 4);
        ISO_CHECK(rb_successor(t, 5, &out) && out == 6);
        rb_free(t);
    }

    /* Delete every element one at a time, verifying the invariant each step. */
    {
        int vs[] = {50, 30, 70, 20, 40, 60, 80, 35, 45, 10, 90, 25};
        int order[] = {70, 20, 50, 90, 30, 10, 80, 40, 60, 25, 45, 35};
        size_t n = 12;
        size_t i;
        RBTree *t = build(vs, n);
        ISO_CHECK_EQ_UINT(rb_size(t), n);
        ISO_CHECK(rb_is_valid_rb(t));
        for (i = 0; i < n; i++) {
            RBTree *d = rb_delete(t, order[i]);
            rb_free(t);
            t = d;
            ISO_CHECK(!rb_contains(t, order[i]));
            ISO_CHECK(rb_is_valid_rb(t));
            ISO_CHECK_EQ_UINT(rb_size(t), n - i - 1);
        }
        ISO_CHECK_EQ_UINT(rb_size(t), 0u);
        rb_free(t);
    }

    /* Duplicate insert keeps set semantics. */
    {
        int vs[] = {5, 5, 5};
        RBTree *t = build(vs, 3);
        ISO_CHECK_EQ_UINT(rb_size(t), 1u);
        ISO_CHECK(rb_is_valid_rb(t));
        rb_free(t);
    }

    /* Larger stress: insert 0..199 ascending (a worst case for a plain BST),
     * confirm balance and order statistics, then delete evens. */
    {
        int i;
        int out = 0;
        int sorted[200];
        size_t written;
        RBTree *t = rb_empty();
        for (i = 0; i < 200; i++) {
            RBTree *n = rb_insert(t, i);
            rb_free(t);
            t = n;
        }
        ISO_CHECK_EQ_UINT(rb_size(t), 200u);
        ISO_CHECK(rb_is_valid_rb(t));
        ISO_CHECK(rb_kth_smallest(t, 100, &out) && out == 99);
        written = rb_to_sorted_array(t, sorted, 200);
        ISO_CHECK_EQ_UINT(written, 200u);
        ISO_CHECK_EQ_INT(sorted[0], 0);
        ISO_CHECK_EQ_INT(sorted[199], 199);

        for (i = 0; i < 200; i += 2) {
            RBTree *n = rb_delete(t, i);
            rb_free(t);
            t = n;
        }
        ISO_CHECK_EQ_UINT(rb_size(t), 100u);
        ISO_CHECK(rb_is_valid_rb(t));
        ISO_CHECK(!rb_contains(t, 100));
        ISO_CHECK(rb_contains(t, 101));
        rb_free(t);
    }

    return ISO_TEST_RESULT();
}
