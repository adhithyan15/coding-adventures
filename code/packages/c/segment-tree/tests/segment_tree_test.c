/* Tests for the C segment tree, using the header-only iso_test.h harness.
 * Covers sum/min/max trees, range queries, point updates, edge ranges, and the
 * safe out-of-range / empty behavior. */
#include "iso_test.h"

#include "segment_tree.h"

int main(void) {
    const int values[6] = {1, 3, 5, 7, 9, 11};
    segment_tree t;

    /* --- sum tree --- */
    ISO_CHECK(segment_tree_init_sum(&t, values, 6));
    ISO_CHECK_EQ_UINT(segment_tree_len(&t), 6);
    ISO_CHECK(!segment_tree_is_empty(&t));
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 5), 36); /* whole array */
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 1, 3), 15); /* 3+5+7 */
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 2, 2), 5);  /* single element */
    /* update element 2: 5 → 10, so [1,3] becomes 3+10+7 = 20 */
    segment_tree_update(&t, 2, 10);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 1, 3), 20);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 5), 41);
    /* out-of-range / inverted ranges return the identity (0), never OOB */
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 3, 2), 0);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 99), 0);
    segment_tree_free(&t);

    /* --- min tree --- */
    ISO_CHECK(segment_tree_init_min(&t, values, 6));
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 5), 1);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 2, 4), 5);
    segment_tree_update(&t, 4, -2); /* element 4: 9 → -2 */
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 2, 4), -2);
    segment_tree_free(&t);

    /* --- max tree --- */
    ISO_CHECK(segment_tree_init_max(&t, values, 6));
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 5), 11);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 2), 5);
    segment_tree_update(&t, 0, 100);
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 2), 100);
    segment_tree_free(&t);

    /* --- empty tree --- */
    ISO_CHECK(segment_tree_init_sum(&t, values, 0));
    ISO_CHECK(segment_tree_is_empty(&t));
    ISO_CHECK_EQ_INT(segment_tree_query(&t, 0, 0), 0); /* identity */
    segment_tree_update(&t, 0, 5);                     /* ignored, no crash */
    segment_tree_free(&t);

    return ISO_TEST_RESULT();
}
