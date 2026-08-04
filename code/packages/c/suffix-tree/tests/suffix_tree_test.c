/* Tests for the C suffix index, using the iso_test.h harness. Pinned to the Rust
 * crate's own assertions (banana / LCS), plus edge cases. */
#include "iso_test.h"

#include <string.h>

#include "suffix_tree.h"

int main(void) {
    /* build("banana"): search / count / node_count. */
    {
        suffix_tree *t = suffix_tree_build("banana", 6);
        size_t pos[8];
        size_t count;
        ISO_CHECK(t != NULL);

        count = suffix_tree_search(t, "ana", 3, pos, 8);
        ISO_CHECK_EQ_UINT(count, 2);
        ISO_CHECK_EQ_UINT(pos[0], 1);
        ISO_CHECK_EQ_UINT(pos[1], 3);
        ISO_CHECK_EQ_UINT(suffix_tree_count_occurrences(t, "ana", 3), 2);
        ISO_CHECK_EQ_UINT(suffix_tree_node_count(t), 7);
        ISO_CHECK_EQ_UINT(suffix_tree_text_len(t), 6);

        suffix_tree_free(t);
    }

    /* Longest repeated substring of "banana" is "ana". */
    {
        suffix_tree *t = suffix_tree_build("banana", 6);
        char buf[16];
        size_t n;
        ISO_CHECK(t != NULL);
        n = suffix_tree_longest_repeated_substring(t, buf, sizeof buf);
        ISO_CHECK_EQ_UINT(n, 3);
        buf[n] = '\0';
        ISO_CHECK_STR_EQ(buf, "ana");
        suffix_tree_free(t);
    }

    /* all_suffixes[0] is the whole text; suffix i is text[i..]. */
    {
        suffix_tree *t = suffix_tree_build("banana", 6);
        const char *p;
        size_t sl;
        ISO_CHECK(t != NULL);
        ISO_CHECK(suffix_tree_suffix(t, 0, &p, &sl));
        ISO_CHECK_EQ_UINT(sl, 6);
        ISO_CHECK(memcmp(p, "banana", 6) == 0);
        ISO_CHECK(suffix_tree_suffix(t, 3, &p, &sl));
        ISO_CHECK_EQ_UINT(sl, 3);
        ISO_CHECK(memcmp(p, "ana", 3) == 0);
        ISO_CHECK(!suffix_tree_suffix(t, 6, &p, &sl)); /* out of range */
        suffix_tree_free(t);
    }

    /* Longest common substring (a free function; the crate's DP vector). */
    {
        char buf[16];
        size_t n = suffix_longest_common_substring("xabxac", 6, "abcabxabcd", 10,
                                                   buf, sizeof buf);
        ISO_CHECK_EQ_UINT(n, 4);
        buf[n] = '\0';
        ISO_CHECK_STR_EQ(buf, "abxa");

        /* Empty inputs give an empty result. */
        ISO_CHECK_EQ_UINT(
            suffix_longest_common_substring("", 0, "abc", 3, buf, sizeof buf), 0);
        ISO_CHECK_EQ_UINT(
            suffix_longest_common_substring("abc", 3, "", 0, buf, sizeof buf), 0);
    }

    /* Edge cases: empty pattern matches everywhere; over-long pattern matches
     * nowhere; missing pattern. */
    {
        suffix_tree *t = suffix_tree_build("abc", 3);
        size_t pos[8];
        ISO_CHECK(t != NULL);
        /* Empty pattern → positions 0..=3 (four of them). */
        ISO_CHECK_EQ_UINT(suffix_tree_search(t, "", 0, pos, 8), 4);
        ISO_CHECK_EQ_UINT(pos[0], 0);
        ISO_CHECK_EQ_UINT(pos[3], 3);
        /* Pattern longer than text. */
        ISO_CHECK_EQ_UINT(suffix_tree_search(t, "abcd", 4, pos, 8), 0);
        /* Absent pattern. */
        ISO_CHECK_EQ_UINT(suffix_tree_count_occurrences(t, "xyz", 3), 0);
        /* Truncated output buffer still returns the full count. */
        ISO_CHECK_EQ_UINT(suffix_tree_search(t, "", 0, pos, 2), 4);
        suffix_tree_free(t);
    }

    /* Empty text. */
    {
        suffix_tree *t = suffix_tree_build("", 0);
        ISO_CHECK(t != NULL);
        ISO_CHECK_EQ_UINT(suffix_tree_node_count(t), 1);
        ISO_CHECK_EQ_UINT(suffix_tree_count_occurrences(t, "a", 1), 0);
        suffix_tree_free(t);
    }

    return ISO_TEST_RESULT();
}
