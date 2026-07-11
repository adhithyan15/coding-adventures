/* Tests for the C radix tree, using the iso_test.h harness. Mirrors the Rust
 * crate's tests: split cases, prune/merge on delete, prefix queries, and sorted
 * key enumeration. */
#include "iso_test.h"

#include <string.h>

#include "radix_tree.h"

/* Collector: gathers visited keys (in order) into a fixed buffer. */
typedef struct {
    char keys[32][32];
    size_t count;
} keylist;

static void collect_cb(const char *key, size_t len, void *user) {
    keylist *kl = (keylist *)user;
    if (kl->count < 32 && len < 32) {
        memcpy(kl->keys[kl->count], key, len + 1); /* includes NUL */
    }
    kl->count++;
}

int main(void) {
    /* Insert / search covering the edge-split cases. */
    {
        radix_tree *t = radix_new();
        long v = 0;
        ISO_CHECK(t != NULL);
        ISO_CHECK(radix_insert(t, "application", 1));
        ISO_CHECK(radix_insert(t, "apple", 2));
        ISO_CHECK(radix_insert(t, "app", 3));
        ISO_CHECK(radix_insert(t, "apt", 4));
        ISO_CHECK(radix_search(t, "application", &v) && v == 1);
        ISO_CHECK(radix_search(t, "apple", &v) && v == 2);
        ISO_CHECK(radix_search(t, "app", &v) && v == 3);
        ISO_CHECK(radix_search(t, "apt", &v) && v == 4);
        ISO_CHECK(!radix_search(t, "appl", &v)); /* a prefix, but not a key */
        ISO_CHECK(radix_contains(t, "app"));
        ISO_CHECK(!radix_contains(t, "appl"));
        ISO_CHECK_EQ_UINT(radix_len(t), 4);
        radix_free(t);
    }

    /* Delete prunes and merges (node_count from the crate's test). */
    {
        radix_tree *t = radix_new();
        long v = 0;
        ISO_CHECK(t != NULL);
        radix_insert(t, "app", 1);
        radix_insert(t, "apple", 2);
        ISO_CHECK_EQ_UINT(radix_node_count(t), 3);
        ISO_CHECK(radix_delete(t, "app"));
        ISO_CHECK(!radix_search(t, "app", &v));
        ISO_CHECK(radix_search(t, "apple", &v) && v == 2);
        ISO_CHECK_EQ_UINT(radix_node_count(t), 2); /* merged */
        ISO_CHECK(!radix_delete(t, "app"));        /* already gone */
        radix_free(t);
    }

    /* starts_with handles mid-edge prefixes. */
    {
        radix_tree *t = radix_new();
        ISO_CHECK(t != NULL);
        radix_insert(t, "searching", 1);
        ISO_CHECK(radix_starts_with(t, "sear"));
        ISO_CHECK(radix_starts_with(t, "search"));
        ISO_CHECK(radix_starts_with(t, "searchin"));
        ISO_CHECK(!radix_starts_with(t, "seek"));
        radix_free(t);
    }

    /* words_with_prefix returns matching keys in sorted order. */
    {
        radix_tree *t = radix_new();
        keylist kl;
        kl.count = 0;
        ISO_CHECK(t != NULL);
        radix_insert(t, "search", 1);
        radix_insert(t, "searcher", 2);
        radix_insert(t, "searching", 3);
        radix_insert(t, "banana", 4);
        radix_words_with_prefix(t, "search", collect_cb, &kl);
        ISO_CHECK_EQ_UINT(kl.count, 3);
        ISO_CHECK_STR_EQ(kl.keys[0], "search");
        ISO_CHECK_STR_EQ(kl.keys[1], "searcher");
        ISO_CHECK_STR_EQ(kl.keys[2], "searching");
        /* Compression: root + "search" + "er" + "ing" + "banana" = 5 nodes. */
        ISO_CHECK_EQ_UINT(radix_node_count(t), 5);
        radix_free(t);
    }

    /* longest_prefix_match returns the most specific stored key. */
    {
        radix_tree *t = radix_new();
        char buf[32];
        long n;
        ISO_CHECK(t != NULL);
        radix_insert(t, "a", 1);
        radix_insert(t, "ab", 2);
        radix_insert(t, "abc", 3);
        radix_insert(t, "application", 4);
        n = radix_longest_prefix_match(t, "abcdef", buf, sizeof buf);
        ISO_CHECK_EQ_INT((int)n, 3);
        buf[n] = '\0';
        ISO_CHECK_STR_EQ(buf, "abc");
        n = radix_longest_prefix_match(t, "application/json", buf, sizeof buf);
        ISO_CHECK_EQ_INT((int)n, 11);
        buf[n] = '\0';
        ISO_CHECK_STR_EQ(buf, "application");
        ISO_CHECK_EQ_INT(
            (int)radix_longest_prefix_match(t, "xyz", buf, sizeof buf), -1);
        radix_free(t);
    }

    /* Empty-string keys are supported. */
    {
        radix_tree *t = radix_new();
        char buf[8];
        long v = 0;
        ISO_CHECK(t != NULL);
        radix_insert(t, "", 1);
        radix_insert(t, "a", 2);
        ISO_CHECK(radix_search(t, "", &v) && v == 1);
        /* Empty key is the longest prefix of anything (root is an end). */
        ISO_CHECK_EQ_INT(
            (int)radix_longest_prefix_match(t, "xyz", buf, sizeof buf), 0);
        ISO_CHECK(radix_delete(t, ""));
        ISO_CHECK(!radix_search(t, "", &v));
        radix_free(t);
    }

    /* keys() enumerates every key in ascending order. */
    {
        radix_tree *t = radix_new();
        keylist kl;
        kl.count = 0;
        ISO_CHECK(t != NULL);
        radix_insert(t, "banana", 1);
        radix_insert(t, "apple", 2);
        radix_insert(t, "apricot", 3);
        radix_insert(t, "app", 4);
        radix_keys(t, collect_cb, &kl);
        ISO_CHECK_EQ_UINT(kl.count, 4);
        ISO_CHECK_STR_EQ(kl.keys[0], "app");
        ISO_CHECK_STR_EQ(kl.keys[1], "apple");
        ISO_CHECK_STR_EQ(kl.keys[2], "apricot");
        ISO_CHECK_STR_EQ(kl.keys[3], "banana");
        radix_free(t);
    }

    /* Overwrite keeps the size; empty tree is empty. */
    {
        radix_tree *t = radix_new();
        long v = 0;
        ISO_CHECK(t != NULL);
        ISO_CHECK(radix_is_empty(t));
        radix_insert(t, "key", 10);
        radix_insert(t, "key", 20); /* overwrite */
        ISO_CHECK_EQ_UINT(radix_len(t), 1);
        ISO_CHECK(radix_search(t, "key", &v) && v == 20);
        radix_free(t);
    }

    return ISO_TEST_RESULT();
}
