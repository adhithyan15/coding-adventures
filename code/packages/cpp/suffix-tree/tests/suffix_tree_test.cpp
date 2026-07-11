// Tests for the C++ suffix index, using the iso_test.h harness. Pinned to the
// Rust crate's own assertions (banana / LCS), plus edge cases.
#include "iso_test.h"

#include <string>
#include <vector>

#include "suffix_tree.hpp"

int main() {
    // build("banana"): search / count / node_count.
    {
        auto t = ca::suffix_tree::build("banana");
        std::vector<std::size_t> pos = t.search("ana");
        ISO_CHECK_EQ_UINT(pos.size(), 2);
        ISO_CHECK_EQ_UINT(pos[0], 1);
        ISO_CHECK_EQ_UINT(pos[1], 3);
        ISO_CHECK_EQ_UINT(t.count_occurrences("ana"), 2);
        ISO_CHECK_EQ_UINT(t.node_count(), 7);
        ISO_CHECK_EQ_UINT(t.text_len(), 6);
    }

    // Longest repeated substring of "banana" is "ana".
    {
        auto t = ca::suffix_tree::build("banana");
        ISO_CHECK(t.longest_repeated_substring() == "ana");
    }

    // all_suffixes[0] is the whole text.
    {
        auto t = ca::suffix_tree::build("banana");
        auto s = t.all_suffixes();
        ISO_CHECK_EQ_UINT(s.size(), 6);
        ISO_CHECK(s[0] == "banana");
        ISO_CHECK(s[3] == "ana");
    }

    // Longest common substring (free function).
    {
        ISO_CHECK(ca::longest_common_substring("xabxac", "abcabxabcd") == "abxa");
        ISO_CHECK(ca::longest_common_substring("", "abc").empty());
        ISO_CHECK(ca::longest_common_substring("abc", "").empty());
        ISO_CHECK(ca::longest_common_substring("abc", "xyz").empty());
    }

    // Edge cases.
    {
        auto t = ca::suffix_tree::build("abc");
        ISO_CHECK_EQ_UINT(t.search("").size(), 4);     // 0..=3
        ISO_CHECK_EQ_UINT(t.search("abcd").size(), 0); // too long
        ISO_CHECK_EQ_UINT(t.count_occurrences("xyz"), 0);
        ISO_CHECK(t.build_ukkonen("z").text() == "z"); // alias for build
    }

    // Empty text.
    {
        auto t = ca::suffix_tree::build("");
        ISO_CHECK_EQ_UINT(t.node_count(), 1);
        ISO_CHECK_EQ_UINT(t.count_occurrences("a"), 0);
        ISO_CHECK(t.all_suffixes().empty());
    }

    return ISO_TEST_RESULT();
}
