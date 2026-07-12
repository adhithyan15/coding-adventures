// Tests for the C++ treap, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's own unit tests (explicit priorities so the
// tree shape is deterministic).
#include "iso_test.h"

#include <optional>
#include <string>
#include <vector>

#include "treap.hpp"

using ca::treap::Treap;

int main() {
    // --- split / merge / search (Rust: split_merge_and_search_work) --------
    {
        Treap<int> t = Treap<int>::empty()
                           .insert(8, 0.8)
                           .insert(3, 0.7)
                           .insert(10, 0.6)
                           .insert(1, 0.9)
                           .insert(6, 0.5);
        ISO_CHECK_EQ_UINT(t.size(), 5u);
        ISO_CHECK(t.contains(6));
        ISO_CHECK(t.is_valid());

        auto parts = t.split(6);
        Treap<int>& left = parts.first;
        Treap<int>& right = parts.second;
        for (int k : left.to_sorted_array()) {
            ISO_CHECK(k <= 6);
        }
        for (int k : right.to_sorted_array()) {
            ISO_CHECK(k > 6);
        }
        ISO_CHECK(left.is_valid());
        ISO_CHECK(right.is_valid());

        Treap<int> merged = Treap<int>::merge(left, right);
        ISO_CHECK(merged.is_valid());
        std::vector<int> want = {1, 3, 6, 8, 10};
        ISO_CHECK(merged.to_sorted_array() == want);

        // split left the original untouched.
        ISO_CHECK_EQ_UINT(t.size(), 5u);
    }

    // --- delete / order statistics (Rust: delete_and_order_statistics) -----
    {
        Treap<int> t = Treap<int>::empty()
                           .insert(8, 0.8)
                           .insert(3, 0.7)
                           .insert(10, 0.6)
                           .insert(1, 0.9)
                           .insert(6, 0.5)
                           .insert(14, 0.4)
                           .insert(4, 0.3)
                           .insert(7, 0.2);
        ISO_CHECK_EQ_UINT(t.size(), 8u);
        ISO_CHECK(t.min_key() == std::optional<int>(1));
        ISO_CHECK(t.max_key() == std::optional<int>(14));
        ISO_CHECK(t.kth_smallest(4) == std::optional<int>(6));
        ISO_CHECK(t.kth_smallest(1) == std::optional<int>(1));
        ISO_CHECK(t.kth_smallest(8) == std::optional<int>(14));
        ISO_CHECK(!t.kth_smallest(0).has_value());
        ISO_CHECK(!t.kth_smallest(9).has_value());
        ISO_CHECK(t.is_valid());

        ISO_CHECK(t.predecessor(6) == std::optional<int>(4));
        ISO_CHECK(t.successor(6) == std::optional<int>(7));
        ISO_CHECK(!t.predecessor(1).has_value());
        ISO_CHECK(!t.successor(14).has_value());

        Treap<int> d = t.erase(3);
        ISO_CHECK(!d.contains(3));
        ISO_CHECK_EQ_UINT(d.size(), 7u);
        ISO_CHECK(d.is_valid());
        // original untouched
        ISO_CHECK(t.contains(3));
        ISO_CHECK_EQ_UINT(t.size(), 8u);

        // deleting a missing key is a no-op.
        Treap<int> d2 = t.erase(999);
        ISO_CHECK_EQ_UINT(d2.size(), 8u);
        ISO_CHECK(d2.is_valid());
    }

    // --- default (PRNG) priorities still build a valid treap ---------------
    {
        Treap<int> t = Treap<int>::empty();
        for (int i = 0; i < 50; ++i) {
            t = t.insert(i * 7 % 50);  // nullopt priority -> PRNG
        }
        ISO_CHECK(t.is_valid());
        ISO_CHECK_EQ_UINT(t.size(), 50u);
        std::vector<int> keys = t.to_sorted_array();
        bool sorted = keys.size() == 50;
        for (std::size_t j = 1; j < keys.size(); ++j) {
            if (keys[j] <= keys[j - 1]) {
                sorted = false;
            }
        }
        ISO_CHECK(sorted);
    }

    // --- empty treap edge cases --------------------------------------------
    {
        Treap<int> t = Treap<int>::empty();
        ISO_CHECK_EQ_UINT(t.size(), 0u);
        ISO_CHECK_EQ_INT(static_cast<int>(t.height()), -1);
        ISO_CHECK(t.is_valid());
        ISO_CHECK(!t.contains(0));
        ISO_CHECK(!t.min_key().has_value());
        ISO_CHECK(!t.max_key().has_value());
        ISO_CHECK(!t.kth_smallest(1).has_value());
        ISO_CHECK(t.to_sorted_array().empty());
    }

    // --- generic over K: works with std::string ---------------------------
    {
        Treap<std::string> t = Treap<std::string>::empty()
                                   .insert("delta", 0.4)
                                   .insert("alpha", 0.9)
                                   .insert("charlie", 0.5)
                                   .insert("bravo", 0.7);
        ISO_CHECK_EQ_UINT(t.size(), 4u);
        ISO_CHECK(t.is_valid());
        std::vector<std::string> want = {"alpha", "bravo", "charlie", "delta"};
        ISO_CHECK(t.to_sorted_array() == want);
        ISO_CHECK(t.min_key() == std::optional<std::string>("alpha"));
        ISO_CHECK(t.kth_smallest(2) == std::optional<std::string>("bravo"));
    }

    return ISO_TEST_RESULT();
}
