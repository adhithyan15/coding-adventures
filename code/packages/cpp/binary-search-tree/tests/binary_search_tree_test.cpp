// Tests for the C++ binary-search-tree, using the header-only iso_test.h harness
// (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <optional>
#include <string>
#include <vector>

#include "binary_search_tree.hpp"

using ca::bst::BST;

// Insert a sequence of values into a fresh tree, returning the final tree.
static BST<int> build_from(const std::vector<int>& vals) {
    BST<int> t = BST<int>::empty();
    for (int v : vals) {
        t = t.insert(v);
    }
    return t;
}

int main() {
    // --- insert / search / order statistics --------------------------------
    {
        BST<int> t = build_from({8, 3, 10, 1, 6, 14, 4, 7});

        ISO_CHECK_EQ_UINT(t.size(), 8u);
        ISO_CHECK(t.contains(4));
        ISO_CHECK(!t.contains(99));

        const BST<int>::Node* node = t.find(4);
        ISO_CHECK(node != nullptr && node->value == 4);
        ISO_CHECK(t.find(99) == nullptr);

        ISO_CHECK(t.min_value() == std::optional<int>(1));
        ISO_CHECK(t.max_value() == std::optional<int>(14));

        // rank(6) = number of values strictly less than 6 = {1,3,4} = 3.
        ISO_CHECK_EQ_UINT(t.rank(6), 3u);
        // kth_smallest(4) (1-based): sorted 1,3,4,6,... -> 4th is 6.
        ISO_CHECK(t.kth_smallest(4) == std::optional<int>(6));
        ISO_CHECK(t.kth_smallest(1) == std::optional<int>(1));
        ISO_CHECK(t.kth_smallest(8) == std::optional<int>(14));
        ISO_CHECK(!t.kth_smallest(0).has_value());
        ISO_CHECK(!t.kth_smallest(9).has_value());

        ISO_CHECK(t.predecessor(6) == std::optional<int>(4));
        ISO_CHECK(t.successor(6) == std::optional<int>(7));
        ISO_CHECK(!t.predecessor(1).has_value());  // none below min
        ISO_CHECK(!t.successor(14).has_value());    // none above max

        ISO_CHECK(t.is_valid());
    }

    // --- persistence: insert returns a new tree, leaves original -----------
    {
        BST<int> t = build_from({5, 3, 8});
        BST<int> t2 = t.insert(1);
        ISO_CHECK_EQ_UINT(t.size(), 3u);  // original untouched
        ISO_CHECK_EQ_UINT(t2.size(), 4u);
        ISO_CHECK(!t.contains(1));
        ISO_CHECK(t2.contains(1));
        // duplicate insert is a no-op (set semantics).
        BST<int> t3 = t2.insert(8);
        ISO_CHECK_EQ_UINT(t3.size(), 4u);
    }

    // --- delete (all three node shapes) ------------------------------------
    {
        BST<int> t = build_from({8, 3, 10, 1, 6, 14, 4, 7});

        // delete a two-child node (3 has children 1 and 6).
        BST<int> d = t.erase(3);
        ISO_CHECK_EQ_UINT(t.size(), 8u);  // original untouched
        ISO_CHECK(!d.contains(3));
        ISO_CHECK_EQ_UINT(d.size(), 7u);
        ISO_CHECK(d.is_valid());

        // delete a leaf (7).
        BST<int> d2 = d.erase(7);
        ISO_CHECK(!d2.contains(7));
        ISO_CHECK_EQ_UINT(d2.size(), 6u);
        ISO_CHECK(d2.is_valid());

        // delete a one-child node (10 has only child 14).
        BST<int> d3 = d.erase(10);
        ISO_CHECK(!d3.contains(10));
        ISO_CHECK(d3.contains(14));
        ISO_CHECK(d3.is_valid());

        // delete a missing value is a no-op.
        BST<int> d4 = d.erase(999);
        ISO_CHECK_EQ_UINT(d4.size(), 7u);
        ISO_CHECK(d4.is_valid());
    }

    // --- from_sorted_array: balanced, in-order round trip ------------------
    {
        std::vector<int> sorted = {1, 2, 3, 4, 5, 6, 7};
        BST<int> t = BST<int>::from_sorted_array(sorted);

        ISO_CHECK_EQ_UINT(t.size(), 7u);
        ISO_CHECK(t.is_valid());
        // 7 nodes balanced -> height 2 (levels 0,1,2).
        ISO_CHECK(t.height() <= 2);

        std::vector<int> out = t.to_sorted_array();
        ISO_CHECK(out == sorted);
    }

    // --- empty tree edge cases ---------------------------------------------
    {
        BST<int> t = BST<int>::empty();
        ISO_CHECK_EQ_UINT(t.size(), 0u);
        ISO_CHECK_EQ_INT(static_cast<int>(t.height()), -1);
        ISO_CHECK(t.is_valid());
        ISO_CHECK(!t.contains(0));
        ISO_CHECK(!t.min_value().has_value());
        ISO_CHECK(!t.max_value().has_value());
        ISO_CHECK(!t.kth_smallest(1).has_value());
        ISO_CHECK_EQ_UINT(t.rank(5), 0u);
        ISO_CHECK(t.to_sorted_array().empty());
    }

    // --- generic over T: works with std::string ----------------------------
    {
        BST<std::string> t = BST<std::string>::empty();
        t = t.insert("delta").insert("alpha").insert("charlie").insert("bravo");
        ISO_CHECK_EQ_UINT(t.size(), 4u);
        std::vector<std::string> want = {"alpha", "bravo", "charlie", "delta"};
        ISO_CHECK(t.to_sorted_array() == want);
        ISO_CHECK(t.min_value() == std::optional<std::string>("alpha"));
        ISO_CHECK(t.kth_smallest(2) == std::optional<std::string>("bravo"));
    }

    return ISO_TEST_RESULT();
}
