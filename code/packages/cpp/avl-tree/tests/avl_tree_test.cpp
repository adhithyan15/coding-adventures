// Tests for the C++ avl-tree, using the iso_test.h harness. Mirrors the Rust
// crate's unit tests and adds delete, predecessor/successor, and persistence
// coverage.
#include "iso_test.h"

#include <cstddef>
#include <vector>

#include "avl_tree.hpp"

using Tree = ca::avl::AVLTree<int>;

int main() {
    // Rust test: rotations_rebalance_the_tree — LL (30,20,10) and RR (10,20,30)
    // both put 20 at the root.
    {
        Tree ll = Tree::empty().insert(30).insert(20).insert(10);
        Tree rr = Tree::empty().insert(10).insert(20).insert(30);
        ISO_CHECK(ll.root() != nullptr && ll.root()->value == 20);
        ISO_CHECK(ll.is_valid_avl());
        ISO_CHECK(rr.root() != nullptr && rr.root()->value == 20);
        ISO_CHECK(rr.is_valid_avl());
    }

    // Rust test: search_and_order_statistics_work.
    {
        Tree t = Tree::empty()
                     .insert(8)
                     .insert(3)
                     .insert(10)
                     .insert(1)
                     .insert(6)
                     .insert(14)
                     .insert(4)
                     .insert(7);
        ISO_CHECK(t.contains(6));
        ISO_CHECK(!t.contains(99));
        ISO_CHECK(t.min_value().value() == 1);
        ISO_CHECK(t.max_value().value() == 14);
        ISO_CHECK_EQ_UINT(t.rank(6), 3u);
        ISO_CHECK(t.kth_smallest(4).value() == 6);
        ISO_CHECK_EQ_UINT(t.size(), 8u);
        ISO_CHECK(t.is_valid_bst());
        ISO_CHECK(t.is_valid_avl());

        std::vector<int> sorted = t.to_sorted_array();
        std::vector<int> expected = {1, 3, 4, 6, 7, 8, 10, 14};
        ISO_CHECK(sorted == expected);
    }

    // Empty-tree edge cases.
    {
        Tree e;
        ISO_CHECK_EQ_INT(static_cast<int>(e.height()), -1);
        ISO_CHECK_EQ_UINT(e.size(), 0u);
        ISO_CHECK(!e.min_value().has_value());
        ISO_CHECK(!e.kth_smallest(1).has_value());
        ISO_CHECK(e.find(5) == nullptr);
        ISO_CHECK(e.is_valid_avl());
    }

    // Predecessor / successor.
    {
        Tree t = Tree::empty()
                     .insert(8).insert(3).insert(10).insert(1)
                     .insert(6).insert(14).insert(4).insert(7);
        ISO_CHECK(t.predecessor(6).value() == 4);
        ISO_CHECK(t.successor(6).value() == 7);
        ISO_CHECK(!t.predecessor(1).has_value());
        ISO_CHECK(!t.successor(14).has_value());
        ISO_CHECK(t.predecessor(5).value() == 4);  // absent query
        ISO_CHECK(t.successor(5).value() == 6);
    }

    // Delete: two-children case, and persistence — the original is untouched.
    {
        Tree t = Tree::empty()
                     .insert(50).insert(30).insert(70).insert(20).insert(40)
                     .insert(60).insert(80).insert(35).insert(45);
        Tree d = t.erase(30);  // 30 has two children
        ISO_CHECK(!d.contains(30));
        ISO_CHECK(d.is_valid_avl());
        ISO_CHECK_EQ_UINT(d.size(), 8u);
        // original unchanged
        ISO_CHECK(t.contains(30));
        ISO_CHECK_EQ_UINT(t.size(), 9u);

        std::vector<int> expected = {20, 35, 40, 45, 50, 60, 70, 80};
        ISO_CHECK(d.to_sorted_array() == expected);

        Tree d2 = t.erase(999);  // absent
        ISO_CHECK_EQ_UINT(d2.size(), 9u);
        ISO_CHECK(d2.is_valid_avl());
    }

    // Duplicate insert keeps set semantics.
    {
        Tree t = Tree::empty().insert(5).insert(5).insert(5);
        ISO_CHECK_EQ_UINT(t.size(), 1u);
        ISO_CHECK(t.is_valid_avl());
    }

    // Value-semantics copy is independent of its source.
    {
        Tree a = Tree::empty().insert(1).insert(2).insert(3);
        Tree b = a;              // deep copy
        Tree c = b.insert(4);    // does not affect a or b
        ISO_CHECK_EQ_UINT(a.size(), 3u);
        ISO_CHECK_EQ_UINT(b.size(), 3u);
        ISO_CHECK_EQ_UINT(c.size(), 4u);
        ISO_CHECK(!b.contains(4));
        ISO_CHECK(c.contains(4));
    }

    // Larger stress: insert 0..99, then delete evens; verify balance throughout.
    {
        Tree t;
        for (int i = 0; i < 100; ++i) {
            t = t.insert(i);
        }
        ISO_CHECK_EQ_UINT(t.size(), 100u);
        ISO_CHECK(t.is_valid_avl());
        ISO_CHECK(t.height() <= 8);
        ISO_CHECK(t.kth_smallest(50).value() == 49);
        ISO_CHECK_EQ_UINT(t.rank(75), 75u);

        for (int i = 0; i < 100; i += 2) {
            t = t.erase(i);
        }
        ISO_CHECK_EQ_UINT(t.size(), 50u);
        ISO_CHECK(t.is_valid_avl());
        ISO_CHECK(!t.contains(42));
        ISO_CHECK(t.contains(43));
    }

    return ISO_TEST_RESULT();
}
