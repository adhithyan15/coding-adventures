/* Tests for the C trie, using the header-only iso_test.h harness. Covers
 * insert/search/contains, delete with pruning, prefix queries, sorted
 * enumeration, and longest-prefix match. */
#include "iso_test.h"

#include <string.h>

#include "trie.h"

/* Collector: append "key=value;" for each visited key so tests can assert the
 * full sorted enumeration in one string. */
typedef struct {
    char buf[512];
} collector;

static void collect_cb(const char *key, int value, void *ud) {
    collector *c = (collector *)ud;
    char piece[64];
    /* value is a small int in these tests; format manually to stay ISO-plain. */
    int n = 0;
    size_t k;
    for (k = 0; key[k] != '\0'; k++) {
        piece[n++] = key[k];
    }
    piece[n++] = '=';
    /* single-digit values in the tests */
    piece[n++] = (char)('0' + value);
    piece[n++] = ';';
    piece[n] = '\0';
    strncat(c->buf, piece, sizeof(c->buf) - strlen(c->buf) - 1);
}

int main(void) {
    trie t;
    int v;
    char keybuf[32];
    collector c;

    ISO_CHECK(trie_init(&t));
    ISO_CHECK(trie_is_empty(&t));
    ISO_CHECK(!trie_contains_key(&t, "cat"));

    /* insert / search / contains. */
    ISO_CHECK(trie_insert(&t, "cat", 1));
    ISO_CHECK(trie_insert(&t, "car", 2));
    ISO_CHECK(trie_insert(&t, "card", 3));
    ISO_CHECK(trie_insert(&t, "dog", 4));
    ISO_CHECK_EQ_UINT(trie_len(&t), 4);
    ISO_CHECK(trie_search(&t, "car", &v));
    ISO_CHECK_EQ_INT(v, 2);
    ISO_CHECK(trie_contains_key(&t, "card"));
    ISO_CHECK(!trie_contains_key(&t, "ca")); /* prefix, not a stored key */
    ISO_CHECK(!trie_search(&t, "ca", &v));

    /* Re-inserting a key overwrites the value without growing size. */
    ISO_CHECK(trie_insert(&t, "cat", 9));
    ISO_CHECK_EQ_UINT(trie_len(&t), 4);
    ISO_CHECK(trie_search(&t, "cat", &v));
    ISO_CHECK_EQ_INT(v, 9);

    /* starts_with. */
    ISO_CHECK(trie_starts_with(&t, "ca"));
    ISO_CHECK(trie_starts_with(&t, "car"));
    ISO_CHECK(!trie_starts_with(&t, "z"));
    ISO_CHECK(trie_starts_with(&t, "")); /* empty prefix, non-empty trie */

    /* Enumeration is in ascending key order. */
    c.buf[0] = '\0';
    ISO_CHECK(trie_foreach(&t, collect_cb, &c));
    ISO_CHECK_STR_EQ(c.buf, "car=2;card=3;cat=9;dog=4;");

    /* Prefix enumeration. */
    c.buf[0] = '\0';
    ISO_CHECK(trie_foreach_prefix(&t, "car", collect_cb, &c));
    ISO_CHECK_STR_EQ(c.buf, "car=2;card=3;");

    /* longest_prefix_match: "cards" → "card"; "ca" → none. */
    ISO_CHECK(trie_longest_prefix_match(&t, "cards", keybuf, sizeof keybuf, &v) ==
              1);
    ISO_CHECK_STR_EQ(keybuf, "card");
    ISO_CHECK_EQ_INT(v, 3);
    ISO_CHECK(trie_longest_prefix_match(&t, "ca", keybuf, sizeof keybuf, &v) == 0);

    /* delete: removing "card" leaves "car"/"cat"/"dog"; the 'd' node is pruned
     * but "car" survives. */
    ISO_CHECK(trie_delete(&t, "card"));
    ISO_CHECK_EQ_UINT(trie_len(&t), 3);
    ISO_CHECK(!trie_contains_key(&t, "card"));
    ISO_CHECK(trie_contains_key(&t, "car"));
    ISO_CHECK(!trie_delete(&t, "card")); /* already gone */
    c.buf[0] = '\0';
    ISO_CHECK(trie_foreach(&t, collect_cb, &c));
    ISO_CHECK_STR_EQ(c.buf, "car=2;cat=9;dog=4;");

    trie_free(&t);
    return ISO_TEST_RESULT();
}
