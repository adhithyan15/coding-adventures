/* Tests for the C treap, using the header-only iso_test.h harness (pure ISO).
 * Vectors mirror the Rust crate's own unit tests (explicit priorities so the
 * tree shape is deterministic). */
#include "iso_test.h"

#include "treap.h"

/* Insert (key, priority) pairs into a fresh treap, freeing intermediates. */
static Treap *build(const int *keys, const double *prios, size_t n) {
    Treap *t = treap_empty();
    size_t i;
    for (i = 0; i < n; i++) {
        Treap *next = treap_insert(t, keys[i], &prios[i]);
        treap_free(t);
        t = next;
    }
    return t;
}

int main(void) {
    /* --- split / merge / search (Rust: split_merge_and_search_work) ----- */
    {
        int keys[] = {8, 3, 10, 1, 6};
        double prios[] = {0.8, 0.7, 0.6, 0.9, 0.5};
        Treap *t = build(keys, prios, 5);
        Treap *left = NULL;
        Treap *right = NULL;
        int buf[8];
        size_t i, n;

        ISO_CHECK_EQ_UINT(treap_size(t), 5u);
        ISO_CHECK(treap_contains(t, 6));
        ISO_CHECK(treap_is_valid(t));

        ISO_CHECK(treap_split(t, 6, &left, &right));
        /* left holds keys <= 6, right holds keys > 6. */
        n = treap_to_sorted_array(left, buf, 8);
        for (i = 0; i < n; i++) {
            ISO_CHECK(buf[i] <= 6);
        }
        n = treap_to_sorted_array(right, buf, 8);
        for (i = 0; i < n; i++) {
            ISO_CHECK(buf[i] > 6);
        }
        ISO_CHECK(treap_is_valid(left));
        ISO_CHECK(treap_is_valid(right));

        {
            Treap *merged = treap_merge(left, right);
            int want[] = {1, 3, 6, 8, 10};
            ISO_CHECK(treap_is_valid(merged));
            n = treap_to_sorted_array(merged, buf, 8);
            ISO_CHECK_EQ_UINT(n, 5u);
            for (i = 0; i < 5; i++) {
                ISO_CHECK_EQ_INT(buf[i], want[i]);
            }
            treap_free(merged);
        }
        /* split/merge left the original untouched. */
        ISO_CHECK_EQ_UINT(treap_size(t), 5u);
        treap_free(left);
        treap_free(right);
        treap_free(t);
    }

    /* --- delete / order statistics (Rust: delete_and_order_statistics) -- */
    {
        int keys[] = {8, 3, 10, 1, 6, 14, 4, 7};
        double prios[] = {0.8, 0.7, 0.6, 0.9, 0.5, 0.4, 0.3, 0.2};
        Treap *t = build(keys, prios, 8);
        int out;

        ISO_CHECK_EQ_UINT(treap_size(t), 8u);
        ISO_CHECK(treap_min_key(t, &out) && out == 1);
        ISO_CHECK(treap_max_key(t, &out) && out == 14);
        ISO_CHECK(treap_kth_smallest(t, 4, &out) && out == 6);
        ISO_CHECK(treap_kth_smallest(t, 1, &out) && out == 1);
        ISO_CHECK(treap_kth_smallest(t, 8, &out) && out == 14);
        ISO_CHECK(!treap_kth_smallest(t, 0, &out));
        ISO_CHECK(!treap_kth_smallest(t, 9, &out));
        ISO_CHECK(treap_is_valid(t));

        /* predecessor / successor. */
        ISO_CHECK(treap_predecessor(t, 6, &out) && out == 4);
        ISO_CHECK(treap_successor(t, 6, &out) && out == 7);
        ISO_CHECK(!treap_predecessor(t, 1, &out));
        ISO_CHECK(!treap_successor(t, 14, &out));

        {
            Treap *d = treap_delete(t, 3);
            ISO_CHECK(!treap_contains(d, 3));
            ISO_CHECK_EQ_UINT(treap_size(d), 7u);
            ISO_CHECK(treap_is_valid(d));
            /* original untouched */
            ISO_CHECK(treap_contains(t, 3));
            ISO_CHECK_EQ_UINT(treap_size(t), 8u);
            treap_free(d);
        }
        /* deleting a missing key is a no-op. */
        {
            Treap *d = treap_delete(t, 999);
            ISO_CHECK_EQ_UINT(treap_size(d), 8u);
            ISO_CHECK(treap_is_valid(d));
            treap_free(d);
        }
        treap_free(t);
    }

    /* --- default (PRNG) priorities still build a valid treap ----------- */
    {
        Treap *t = treap_empty();
        int i;
        for (i = 0; i < 50; i++) {
            Treap *next = treap_insert(t, i * 7 % 50, NULL); /* NULL -> PRNG */
            treap_free(t);
            t = next;
        }
        ISO_CHECK(treap_is_valid(t));
        ISO_CHECK_EQ_UINT(treap_size(t), 50u);
        /* keys come out sorted regardless of priorities. */
        {
            int buf[50];
            size_t n = treap_to_sorted_array(t, buf, 50);
            int ok = 1, j;
            ISO_CHECK_EQ_UINT(n, 50u);
            for (j = 1; j < 50; j++) {
                if (buf[j] <= buf[j - 1]) {
                    ok = 0;
                }
            }
            ISO_CHECK(ok);
        }
        treap_free(t);
    }

    /* --- empty treap edge cases ---------------------------------------- */
    {
        Treap *t = treap_empty();
        int out, buf[4];
        ISO_CHECK_EQ_UINT(treap_size(t), 0u);
        ISO_CHECK_EQ_INT((int)treap_height(t), -1);
        ISO_CHECK(treap_is_valid(t));
        ISO_CHECK(!treap_contains(t, 0));
        ISO_CHECK(!treap_min_key(t, &out));
        ISO_CHECK(!treap_max_key(t, &out));
        ISO_CHECK(!treap_kth_smallest(t, 1, &out));
        ISO_CHECK_EQ_UINT(treap_to_sorted_array(t, buf, 4), 0u);
        treap_free(t);
    }

    return ISO_TEST_RESULT();
}
