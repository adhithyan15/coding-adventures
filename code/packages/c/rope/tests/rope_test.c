/* Tests for the C rope, using the iso_test.h harness. Pinned to the Rust crate's
 * own assertions, plus consuming-API and edge-case coverage. Remember: every
 * function taking a rope* consumes it. */
#include "iso_test.h"

#include <string.h>

#include "rope.h"

/* Compare a rope's contents to a NUL-terminated expected string. */
static int rope_eq(const rope *r, const char *expect) {
    char buf[64];
    size_t n = rope_to_string(r, buf, sizeof buf);
    return n == strlen(expect) && memcmp(buf, expect, n) == 0;
}

int main(void) {
    /* concat / index / split (from the crate's first test). */
    {
        rope *r = rope_concat(rope_from_string("hello", 5),
                              rope_from_string(" world", 6));
        char c = 0;
        rope *left = NULL;
        rope *right = NULL;
        ISO_CHECK(r != NULL);
        ISO_CHECK_EQ_UINT(rope_len(r), 11);
        ISO_CHECK(rope_index(r, 1, &c));
        ISO_CHECK_EQ_INT(c, 'e');
        ISO_CHECK(rope_split(r, 5, &left, &right)); /* consumes r */
        ISO_CHECK(rope_eq(left, "hello"));
        ISO_CHECK(rope_eq(right, " world"));
        rope_free(left);
        rope_free(right);
    }

    /* insert / delete / rebalance (from the crate's second test). */
    {
        rope *r = rope_insert(rope_from_string("ace", 3), 1, "b", 1);
        rope *r2;
        char sub[8];
        size_t sn;
        ISO_CHECK(r != NULL);
        r = rope_insert(r, 3, "d", 1);
        ISO_CHECK(rope_eq(r, "abcde"));
        r = rope_delete(r, 1, 2);
        ISO_CHECK(rope_eq(r, "ade"));
        rope_free(r);

        r2 = rope_rebalance(rope_concat(rope_from_string("ab", 2),
                                        rope_from_string("cdef", 4)));
        ISO_CHECK(r2 != NULL);
        ISO_CHECK(rope_is_balanced(r2));
        ISO_CHECK(rope_depth(r2) <= 3);
        sn = rope_substring(r2, 1, 4, sub, sizeof sub);
        ISO_CHECK_EQ_UINT(sn, 3);
        sub[sn] = '\0';
        ISO_CHECK_STR_EQ(sub, "bcd");
        rope_free(r2);
    }

    /* Empty rope. */
    {
        rope *e = rope_empty();
        char x = 0;
        ISO_CHECK(e != NULL);
        ISO_CHECK(rope_is_empty(e));
        ISO_CHECK_EQ_UINT(rope_len(e), 0);
        ISO_CHECK_EQ_UINT(rope_depth(e), 0);
        ISO_CHECK(rope_is_balanced(e));
        ISO_CHECK(!rope_index(e, 0, &x));
        rope_free(e);
    }

    /* Concat with an empty operand returns the other side. */
    {
        rope *a = rope_concat(rope_from_string("hi", 2), rope_empty());
        rope *b = rope_concat(rope_empty(), rope_from_string("yo", 2));
        ISO_CHECK(rope_eq(a, "hi"));
        ISO_CHECK(rope_eq(b, "yo"));
        rope_free(a);
        rope_free(b);
    }

    /* substring clamping and to_string truncation. */
    {
        rope *s = rope_from_string("abcdef", 6);
        char t[3];
        ISO_CHECK(s != NULL);
        ISO_CHECK_EQ_UINT(rope_substring(s, 4, 100, NULL, 0), 2); /* "ef" */
        ISO_CHECK_EQ_UINT(rope_substring(s, 3, 3, NULL, 0), 0);   /* empty */
        /* to_string returns the full length even when the buffer is short. */
        ISO_CHECK_EQ_UINT(rope_to_string(s, t, sizeof t), 6);
        ISO_CHECK(memcmp(t, "abc", 3) == 0);
        rope_free(s);
    }

    /* delete past the end clamps. */
    {
        rope *d = rope_delete(rope_from_string("hello", 5), 2, 100);
        ISO_CHECK(rope_eq(d, "he"));
        rope_free(d);
    }

    /* Index deep into a concatenated rope uses the weighted descent. */
    {
        rope *r = rope_concat(
            rope_concat(rope_from_string("ab", 2), rope_from_string("cd", 2)),
            rope_from_string("ef", 2));
        char c = 0;
        ISO_CHECK(rope_len(r) == 6);
        ISO_CHECK(rope_index(r, 0, &c) && c == 'a');
        ISO_CHECK(rope_index(r, 3, &c) && c == 'd');
        ISO_CHECK(rope_index(r, 5, &c) && c == 'f');
        ISO_CHECK(!rope_index(r, 6, &c));
        rope_free(r);
    }

    return ISO_TEST_RESULT();
}
