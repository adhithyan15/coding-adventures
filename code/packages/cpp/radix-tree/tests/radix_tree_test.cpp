// Tests for the C++ radix tree, using the iso_test.h harness. Mirrors the Rust
// crate's tests (split cases, prune/merge, prefix queries, sorted enumeration),
// plus a generic string-value check.
#include "iso_test.h"

#include <string>
#include <vector>

#include "radix_tree.hpp"

int main() {
    // Insert / search covering the split cases.
    {
        ca::radix_tree<int> t;
        t.insert("application", 1);
        t.insert("apple", 2);
        t.insert("app", 3);
        t.insert("apt", 4);
        ISO_CHECK(t.search("application") != nullptr && *t.search("application") == 1);
        ISO_CHECK(*t.search("apple") == 2);
        ISO_CHECK(*t.search("app") == 3);
        ISO_CHECK(*t.search("apt") == 4);
        ISO_CHECK(t.search("appl") == nullptr);
        ISO_CHECK(t.contains("app") && !t.contains("appl"));
        ISO_CHECK_EQ_UINT(t.len(), 4);
    }

    // Delete prunes and merges.
    {
        ca::radix_tree<int> t;
        t.insert("app", 1);
        t.insert("apple", 2);
        ISO_CHECK_EQ_UINT(t.node_count(), 3);
        ISO_CHECK(t.remove("app"));
        ISO_CHECK(t.search("app") == nullptr);
        ISO_CHECK(*t.search("apple") == 2);
        ISO_CHECK_EQ_UINT(t.node_count(), 2);
        ISO_CHECK(!t.remove("app"));
    }

    // starts_with handles mid-edge prefixes.
    {
        ca::radix_tree<int> t;
        t.insert("searching", 1);
        ISO_CHECK(t.starts_with("sear"));
        ISO_CHECK(t.starts_with("search"));
        ISO_CHECK(t.starts_with("searchin"));
        ISO_CHECK(!t.starts_with("seek"));
    }

    // words_with_prefix is sorted; compression gives four nodes.
    {
        ca::radix_tree<int> t;
        t.insert("search", 1);
        t.insert("searcher", 2);
        t.insert("searching", 3);
        t.insert("banana", 4);
        auto w = t.words_with_prefix("search");
        ISO_CHECK_EQ_UINT(w.size(), 3);
        ISO_CHECK(w[0] == "search" && w[1] == "searcher" && w[2] == "searching");
    }

    // longest_prefix_match returns the most specific key.
    {
        ca::radix_tree<int> t;
        t.insert("a", 1);
        t.insert("ab", 2);
        t.insert("abc", 3);
        t.insert("application", 4);
        ISO_CHECK(t.longest_prefix_match("abcdef") ==
                  std::optional<std::string>("abc"));
        ISO_CHECK(t.longest_prefix_match("application/json") ==
                  std::optional<std::string>("application"));
        ISO_CHECK(!t.longest_prefix_match("xyz").has_value());
    }

    // Empty-string keys.
    {
        ca::radix_tree<int> t;
        t.insert("", 1);
        t.insert("a", 2);
        ISO_CHECK(t.search("") != nullptr && *t.search("") == 1);
        ISO_CHECK(t.longest_prefix_match("xyz") ==
                  std::optional<std::string>(std::string()));
        ISO_CHECK(t.remove(""));
        ISO_CHECK(t.search("") == nullptr);
    }

    // keys() enumerates in ascending order.
    {
        ca::radix_tree<int> t;
        t.insert("banana", 1);
        t.insert("apple", 2);
        t.insert("apricot", 3);
        t.insert("app", 4);
        auto k = t.keys();
        std::vector<std::string> expect = {"app", "apple", "apricot", "banana"};
        ISO_CHECK(k == expect);
    }

    // Generic string values, with overwrite.
    {
        ca::radix_tree<std::string> t;
        t.insert("greeting", "hello");
        t.insert("greeting", "hi"); // overwrite
        ISO_CHECK_EQ_UINT(t.len(), 1);
        ISO_CHECK(*t.search("greeting") == "hi");
        t.insert("gremlin", "gizmo");
        ISO_CHECK(*t.search("gremlin") == "gizmo");
        ISO_CHECK(*t.search("greeting") == "hi");
    }

    return ISO_TEST_RESULT();
}
