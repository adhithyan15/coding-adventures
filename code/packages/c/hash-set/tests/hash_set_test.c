/* Tests for the C hash set, using the iso_test.h harness. Mirrors the Rust
 * crate's tests: membership, duplicate handling, set algebra, and relations. */
#include "iso_test.h"

#include <string.h>

#include "hash_set.h"

static int add(hashset *s, const char *e) { return hashset_add(s, e, strlen(e)); }
static int has(const hashset *s, const char *e) {
    return hashset_contains(s, e, strlen(e));
}

/* Counter callback for hashset_for_each. */
static void count_cb(const void *e, size_t n, void *user) {
    (void)e;
    (void)n;
    (*(size_t *)user)++;
}

int main(void) {
    /* Basic membership. */
    {
        hashset *s = hashset_new();
        ISO_CHECK(s != NULL);
        ISO_CHECK(add(s, "one"));
        ISO_CHECK(add(s, "two"));
        ISO_CHECK(add(s, "three"));
        ISO_CHECK_EQ_UINT(hashset_size(s), 3);
        ISO_CHECK(has(s, "one"));
        ISO_CHECK(!has(s, "four"));
        ISO_CHECK(!hashset_is_empty(s));
        hashset_free(s);
    }

    /* Duplicates are ignored. */
    {
        hashset *s = hashset_new();
        ISO_CHECK(s != NULL);
        add(s, "x");
        add(s, "x");
        add(s, "y");
        add(s, "y");
        add(s, "z");
        ISO_CHECK_EQ_UINT(hashset_size(s), 3);
        hashset_free(s);
    }

    /* Remove, and a redundant remove. */
    {
        hashset *s = hashset_new();
        size_t count = 0;
        ISO_CHECK(s != NULL);
        add(s, "a");
        add(s, "b");
        ISO_CHECK(hashset_remove(s, "b", 1));
        ISO_CHECK(!has(s, "b"));
        ISO_CHECK(!hashset_remove(s, "b", 1));
        ISO_CHECK_EQ_UINT(hashset_size(s), 1);
        hashset_for_each(s, count_cb, &count);
        ISO_CHECK_EQ_UINT(count, 1);
        hashset_free(s);
    }

    /* Set algebra: A={1..5}, B={3..7}. */
    {
        hashset *a = hashset_new();
        hashset *b = hashset_new();
        hashset *u, *i, *d, *sd;
        ISO_CHECK(a != NULL && b != NULL);
        add(a, "1");
        add(a, "2");
        add(a, "3");
        add(a, "4");
        add(a, "5");
        add(b, "3");
        add(b, "4");
        add(b, "5");
        add(b, "6");
        add(b, "7");

        u = hashset_union(a, b);
        i = hashset_intersection(a, b);
        d = hashset_difference(a, b);
        sd = hashset_symmetric_difference(a, b);
        ISO_CHECK(u != NULL && i != NULL && d != NULL && sd != NULL);

        ISO_CHECK_EQ_UINT(hashset_size(u), 7); /* {1..7} */
        ISO_CHECK_EQ_UINT(hashset_size(i), 3); /* {3,4,5} */
        ISO_CHECK_EQ_UINT(hashset_size(d), 2); /* {1,2} */
        ISO_CHECK_EQ_UINT(hashset_size(sd), 4); /* {1,2,6,7} */

        ISO_CHECK(has(i, "3") && has(i, "4") && has(i, "5"));
        ISO_CHECK(has(d, "1") && has(d, "2") && !has(d, "3"));
        ISO_CHECK(has(sd, "1") && has(sd, "6") && !has(sd, "3"));

        hashset_free(u);
        hashset_free(i);
        hashset_free(d);
        hashset_free(sd);
        hashset_free(a);
        hashset_free(b);
    }

    /* Relations: A={1,2,3}, B={1,2,3,4,5}, C={10,20}. */
    {
        hashset *a = hashset_new();
        hashset *b = hashset_new();
        hashset *c = hashset_new();
        hashset *a2 = hashset_new();
        ISO_CHECK(a != NULL && b != NULL && c != NULL && a2 != NULL);
        add(a, "1");
        add(a, "2");
        add(a, "3");
        add(b, "1");
        add(b, "2");
        add(b, "3");
        add(b, "4");
        add(b, "5");
        add(c, "10");
        add(c, "20");
        add(a2, "1");
        add(a2, "2");
        add(a2, "3");

        ISO_CHECK(hashset_is_subset(a, b));
        ISO_CHECK(!hashset_is_subset(b, a));
        ISO_CHECK(hashset_is_superset(b, a));
        ISO_CHECK(hashset_is_disjoint(a, c));
        ISO_CHECK(!hashset_is_disjoint(a, b));
        ISO_CHECK(hashset_equals(a, a2));
        ISO_CHECK(!hashset_equals(a, b));

        hashset_free(a);
        hashset_free(b);
        hashset_free(c);
        hashset_free(a2);
    }

    /* Works with a non-default map configuration too (open addressing + djb2). */
    {
        hashset *s =
            hashset_new_with(4, HASHMAP_OPEN_ADDRESSING, HASHMAP_DJB2);
        ISO_CHECK(s != NULL);
        add(s, "alpha");
        add(s, "beta");
        ISO_CHECK(has(s, "alpha"));
        ISO_CHECK(has(s, "beta"));
        ISO_CHECK(!has(s, "gamma"));
        hashset_free(s);
    }

    return ISO_TEST_RESULT();
}
