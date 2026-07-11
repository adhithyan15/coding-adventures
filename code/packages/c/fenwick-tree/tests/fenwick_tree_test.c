/* Tests for the C Fenwick tree, using the header-only iso_test.h harness.
 * Mirrors the Rust crate: build from a slice, point/prefix/range queries,
 * updates, find_kth cumulative search, and the error paths. */
#include "iso_test.h"

#include "fenwick_tree.h"

int main(void) {
    /* Prefix sums of [3, 2, -1, 6, 5, 4, -3, 3] (1-based). */
    const double values[8] = {3, 2, -1, 6, 5, 4, -3, 3};
    fenwick_tree t;
    double out;
    size_t k;

    ISO_CHECK_EQ_INT(fenwick_init_from_slice(&t, values, 8), FENWICK_OK);
    ISO_CHECK_EQ_UINT(fenwick_len(&t), 8);
    ISO_CHECK(!fenwick_is_empty(&t));

    /* prefix_sum(0) is the empty prefix. */
    ISO_CHECK_EQ_INT(fenwick_prefix_sum(&t, 0, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 0.0, 1e-9);
    /* prefix_sum(5) = 3+2-1+6+5 = 15. */
    ISO_CHECK_EQ_INT(fenwick_prefix_sum(&t, 5, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 15.0, 1e-9);
    /* full sum = 19. */
    ISO_CHECK_EQ_INT(fenwick_prefix_sum(&t, 8, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 19.0, 1e-9);

    /* point_query recovers individual elements. */
    ISO_CHECK_EQ_INT(fenwick_point_query(&t, 4, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 6.0, 1e-9);
    ISO_CHECK_EQ_INT(fenwick_point_query(&t, 3, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, -1.0, 1e-9);

    /* range_sum(3, 6) = -1+6+5+4 = 14. */
    ISO_CHECK_EQ_INT(fenwick_range_sum(&t, 3, 6, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 14.0, 1e-9);

    /* update(3, +5): element 3 goes -1 → 4, so range_sum(3,6) → 19. */
    ISO_CHECK_EQ_INT(fenwick_update(&t, 3, 5.0), FENWICK_OK);
    ISO_CHECK_EQ_INT(fenwick_point_query(&t, 3, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 4.0, 1e-9);
    ISO_CHECK_EQ_INT(fenwick_range_sum(&t, 3, 6, &out), FENWICK_OK);
    ISO_CHECK_EQ_DBL(out, 19.0, 1e-9);

    /* Error paths. */
    ISO_CHECK_EQ_INT(fenwick_update(&t, 0, 1.0), FENWICK_INDEX_OUT_OF_RANGE);
    ISO_CHECK_EQ_INT(fenwick_update(&t, 9, 1.0), FENWICK_INDEX_OUT_OF_RANGE);
    ISO_CHECK_EQ_INT(fenwick_prefix_sum(&t, 9, &out), FENWICK_INDEX_OUT_OF_RANGE);
    ISO_CHECK_EQ_INT(fenwick_range_sum(&t, 5, 3, &out), FENWICK_INVALID_RANGE);

    fenwick_free(&t);

    /* find_kth on a non-negative frequency table [1, 3, 2, 4] (cumulative
     * 1,4,6,10): the smallest index whose prefix sum reaches the target. */
    {
        const double freq[4] = {1, 3, 2, 4};
        ISO_CHECK_EQ_INT(fenwick_init_from_slice(&t, freq, 4), FENWICK_OK);
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 1.0, &k), FENWICK_OK);
        ISO_CHECK_EQ_UINT(k, 1); /* prefix(1)=1 >= 1 */
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 4.0, &k), FENWICK_OK);
        ISO_CHECK_EQ_UINT(k, 2); /* prefix(2)=4 >= 4 */
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 5.0, &k), FENWICK_OK);
        ISO_CHECK_EQ_UINT(k, 3); /* prefix(3)=6 >= 5 */
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 10.0, &k), FENWICK_OK);
        ISO_CHECK_EQ_UINT(k, 4);
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 0.0, &k),
                         FENWICK_NON_POSITIVE_TARGET);
        ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 11.0, &k),
                         FENWICK_TARGET_EXCEEDS_TOTAL);
        fenwick_free(&t);
    }

    /* Empty tree. */
    ISO_CHECK_EQ_INT(fenwick_init(&t, 0), FENWICK_OK);
    ISO_CHECK(fenwick_is_empty(&t));
    ISO_CHECK_EQ_INT(fenwick_find_kth(&t, 1.0, &k), FENWICK_EMPTY_TREE);
    fenwick_free(&t);

    return ISO_TEST_RESULT();
}
