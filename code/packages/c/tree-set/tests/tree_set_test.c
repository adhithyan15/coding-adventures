/* Tests for the C tree-set, using the iso_test.h harness. Mirrors the Rust
 * crate's unit tests (ordered-set operations + set algebra) and adds
 * persistence and relation checks. */
#include "iso_test.h"

#include <stdlib.h> /* NULL */

#include "tree_set.h"

/* Compare the set's sorted contents against `expected` (length n). */
static int matches(const TreeSet *s, const int *expected, size_t n) {
    int buf[64];
    size_t written = tset_to_sorted_array(s, buf, 64);
    size_t i;
    if (written != n) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        if (buf[i] != expected[i]) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    /* Rust test: avl_backend_supports_ordered_set_operations.
     * from_list([7,3,9,1,5,3]) collapses the duplicate 3. */
    {
        int vs[] = {7, 3, 9, 1, 5, 3};
        int expected[] = {1, 3, 5, 7, 9};
        int out = 0;
        int rbuf[8];
        int r_incl[] = {3, 5, 7};
        int r_excl[] = {5};
        size_t rn;
        TreeSet *set = tset_from_array(vs, 6);
        TreeSet *removed;

        ISO_CHECK(matches(set, expected, 5));
        ISO_CHECK_EQ_UINT(tset_size(set), 5u);
        ISO_CHECK(tset_min_value(set, &out) && out == 1);
        ISO_CHECK(tset_max_value(set, &out) && out == 9);
        ISO_CHECK_EQ_UINT(tset_rank(set, 7), 3u);
        ISO_CHECK(tset_kth_smallest(set, 3, &out) && out == 5);

        rn = tset_range(set, 3, 7, 1, rbuf, 8);
        ISO_CHECK_EQ_UINT(rn, 3u);
        ISO_CHECK(rbuf[0] == r_incl[0] && rbuf[1] == r_incl[1] &&
                  rbuf[2] == r_incl[2]);
        rn = tset_range(set, 3, 7, 0, rbuf, 8);
        ISO_CHECK_EQ_UINT(rn, 1u);
        ISO_CHECK(rbuf[0] == r_excl[0]);

        /* remove is persistent — the original keeps all five. */
        removed = tset_remove(set, 5);
        {
            int after[] = {1, 3, 7, 9};
            ISO_CHECK(matches(removed, after, 4));
        }
        ISO_CHECK(matches(set, expected, 5));
        tset_free(removed);
        tset_free(set);
    }

    /* Rust test: avl_backend_set_algebra_works. */
    {
        int lvs[] = {1, 2, 3, 5};
        int rvs[] = {3, 4, 5, 6};
        int e12[] = {1, 2};
        TreeSet *left = tset_from_array(lvs, 4);
        TreeSet *right = tset_from_array(rvs, 4);
        TreeSet *u = tset_union(left, right);
        TreeSet *in = tset_intersection(left, right);
        TreeSet *df = tset_difference(left, right);
        TreeSet *sd = tset_symmetric_difference(left, right);
        TreeSet *set12 = tset_from_array(e12, 2);

        {
            int eu[] = {1, 2, 3, 4, 5, 6};
            int ei[] = {3, 5};
            int ed[] = {1, 2};
            int es[] = {1, 2, 4, 6};
            ISO_CHECK(matches(u, eu, 6));
            ISO_CHECK(matches(in, ei, 2));
            ISO_CHECK(matches(df, ed, 2));
            ISO_CHECK(matches(sd, es, 4));
        }

        ISO_CHECK(tset_is_subset(left, u));       /* left ⊆ left∪right */
        ISO_CHECK(tset_is_superset(u, left));     /* symmetric */
        ISO_CHECK(!tset_is_subset(u, left));
        {
            /* left ⊇ (left∩right) ∪ {1,2} */
            TreeSet *iu = tset_union(in, set12);
            ISO_CHECK(tset_is_superset(left, iu));
            tset_free(iu);
        }
        {
            int disj[] = {8, 9};
            TreeSet *d = tset_from_array(disj, 2);
            ISO_CHECK(tset_is_disjoint(left, d));
            ISO_CHECK(!tset_is_disjoint(left, right));
            tset_free(d);
        }
        {
            int same[] = {1, 2, 3, 5};
            TreeSet *eq = tset_from_array(same, 4);
            ISO_CHECK(tset_equals(left, eq));
            ISO_CHECK(!tset_equals(left, right));
            tset_free(eq);
        }

        tset_free(left);
        tset_free(right);
        tset_free(u);
        tset_free(in);
        tset_free(df);
        tset_free(sd);
        tset_free(set12);
    }

    /* Predecessor / successor / membership, matching the RB-backend Rust test
     * (our C port uses the AVL backend, but the observable API is identical). */
    {
        int vs[] = {10, 4, 14, 2, 8, 12, 16};
        int out = 0;
        TreeSet *set = tset_from_array(vs, 7);
        TreeSet *d;
        int expected[] = {2, 4, 8, 10, 12, 14, 16};
        ISO_CHECK(matches(set, expected, 7));
        ISO_CHECK(tset_predecessor(set, 10, &out) && out == 8);
        ISO_CHECK(tset_successor(set, 10, &out) && out == 12);
        ISO_CHECK(tset_contains(set, 14));
        d = tset_remove(set, 8); /* persistent remove */
        {
            int after[] = {2, 4, 10, 12, 14, 16};
            ISO_CHECK(matches(d, after, 6));
        }
        ISO_CHECK(tset_contains(set, 8)); /* original untouched */
        tset_free(d);
        tset_free(set);
    }

    /* Edge cases: empty set, range with min > max, empty algebra. */
    {
        TreeSet *e = tset_empty();
        TreeSet *e2 = tset_empty();
        TreeSet *u = tset_union(e, e2);
        int out = 42;
        int buf[4];
        ISO_CHECK(tset_is_empty(e));
        ISO_CHECK_EQ_UINT(tset_size(e), 0u);
        ISO_CHECK(!tset_min_value(e, &out) && out == 42);
        ISO_CHECK_EQ_UINT(tset_range(e, 1, 10, 1, buf, 4), 0u);
        ISO_CHECK(tset_is_empty(u));
        ISO_CHECK(tset_is_subset(e, e2)); /* empty ⊆ empty */
        ISO_CHECK(tset_is_disjoint(e, e2));
        ISO_CHECK(tset_equals(e, e2));
        {
            int one[] = {5};
            TreeSet *s = tset_from_array(one, 1);
            /* range with min > max is empty */
            ISO_CHECK_EQ_UINT(tset_range(s, 10, 1, 1, buf, 4), 0u);
            tset_free(s);
        }
        tset_free(e);
        tset_free(e2);
        tset_free(u);
    }

    return ISO_TEST_RESULT();
}
