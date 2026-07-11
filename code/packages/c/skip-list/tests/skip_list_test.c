/* Tests for the C skip-list (ordered map), using the iso_test.h harness. Covers
 * insert/overwrite/search/delete, order statistics (rank/by_rank), min/max,
 * ordered enumeration, range queries, and the reported parameters. */
#include "iso_test.h"

#include "skip_list.h"

/* Collector: append "k:v " for each visited entry so tests can assert order. */
typedef struct {
    char buf[256];
    size_t n;
} collector;

static void collect_cb(int key, int value, void *ud) {
    collector *c = (collector *)ud;
    /* small non-negative ints in these tests → single digits */
    if (c->n + 4 < sizeof c->buf) {
        c->buf[c->n++] = (char)('0' + key);
        c->buf[c->n++] = ':';
        c->buf[c->n++] = (char)('0' + value);
        c->buf[c->n++] = ' ';
        c->buf[c->n] = '\0';
    }
}

int main(void) {
    skiplist s;
    int v;
    size_t r;
    collector c;

    ISO_CHECK(skiplist_init(&s));
    ISO_CHECK(skiplist_is_empty(&s));
    ISO_CHECK_EQ_UINT(skiplist_max_level(&s), 32);
    ISO_CHECK_EQ_DBL(skiplist_probability(&s), 0.5, 1e-12);
    ISO_CHECK_EQ_UINT(skiplist_current_max(&s), 1); /* empty */
    ISO_CHECK(!skiplist_search(&s, 5, &v));

    /* Insert out of order (single-digit values so the collector can print them);
     * the map keeps keys sorted. */
    ISO_CHECK(skiplist_insert(&s, 5, 5));
    ISO_CHECK(skiplist_insert(&s, 1, 1));
    ISO_CHECK(skiplist_insert(&s, 9, 9));
    ISO_CHECK(skiplist_insert(&s, 3, 3));
    ISO_CHECK(skiplist_insert(&s, 7, 7));
    ISO_CHECK_EQ_UINT(skiplist_len(&s), 5);

    /* search / contains. */
    ISO_CHECK(skiplist_search(&s, 7, &v));
    ISO_CHECK_EQ_INT(v, 7);
    ISO_CHECK(skiplist_contains(&s, 3));
    ISO_CHECK(!skiplist_contains(&s, 4));

    /* Overwrite keeps size, updates value (3 → 8). */
    ISO_CHECK(skiplist_insert(&s, 3, 8));
    ISO_CHECK_EQ_UINT(skiplist_len(&s), 5);
    ISO_CHECK(skiplist_search(&s, 3, &v));
    ISO_CHECK_EQ_INT(v, 8);

    /* Order statistics: keys sorted are 1,3,5,7,9 at ranks 0..4. */
    ISO_CHECK(skiplist_rank(&s, 5, &r));
    ISO_CHECK_EQ_UINT(r, 2);
    ISO_CHECK(!skiplist_rank(&s, 4, &r)); /* absent */
    ISO_CHECK(skiplist_by_rank(&s, 0, &v));
    ISO_CHECK_EQ_INT(v, 1);
    ISO_CHECK(skiplist_by_rank(&s, 4, &v));
    ISO_CHECK_EQ_INT(v, 9);
    ISO_CHECK(!skiplist_by_rank(&s, 5, &v)); /* out of range */

    /* min / max. */
    ISO_CHECK(skiplist_min(&s, &v));
    ISO_CHECK_EQ_INT(v, 1);
    ISO_CHECK(skiplist_max(&s, &v));
    ISO_CHECK_EQ_INT(v, 9);

    /* Ordered enumeration. */
    c.n = 0;
    c.buf[0] = '\0';
    skiplist_foreach(&s, collect_cb, &c);
    ISO_CHECK_STR_EQ(c.buf, "1:1 3:8 5:5 7:7 9:9 ");

    /* Inclusive range [3,7] → 3,5,7. */
    c.n = 0;
    c.buf[0] = '\0';
    skiplist_range(&s, 3, 7, 1, collect_cb, &c);
    ISO_CHECK_STR_EQ(c.buf, "3:8 5:5 7:7 ");
    /* Exclusive range (3,7) → 5 only. */
    c.n = 0;
    c.buf[0] = '\0';
    skiplist_range(&s, 3, 7, 0, collect_cb, &c);
    ISO_CHECK_STR_EQ(c.buf, "5:5 ");
    /* Inverted range → empty. */
    c.n = 0;
    c.buf[0] = '\0';
    skiplist_range(&s, 7, 3, 1, collect_cb, &c);
    ISO_CHECK_STR_EQ(c.buf, "");

    /* delete removes and shifts. */
    ISO_CHECK(skiplist_delete(&s, 5));
    ISO_CHECK_EQ_UINT(skiplist_len(&s), 4);
    ISO_CHECK(!skiplist_contains(&s, 5));
    ISO_CHECK(!skiplist_delete(&s, 5)); /* already gone */
    ISO_CHECK(skiplist_rank(&s, 7, &r));
    ISO_CHECK_EQ_UINT(r, 2); /* 1,3,7,9 → 7 at rank 2 */

    /* current_max grows with size (ceil(log2 len), clamped to max_level). */
    ISO_CHECK(skiplist_current_max(&s) >= 1);

    skiplist_free(&s);
    return ISO_TEST_RESULT();
}
