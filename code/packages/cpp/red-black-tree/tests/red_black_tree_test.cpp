// Tests for the C++ red-black-tree, using the iso_test.h harness. Mirrors the
// Rust crate's unit tests and adds delete, neighbour queries, value-semantics
// persistence, and a stress that re-verifies the LLRB invariant throughout.
#include "iso_test.h"

#include <cstddef>
#include <vector>

#include "red_black_tree.hpp"

using Tree = ca::rb::RBTree<int>;
using Color = Tree::Color;

int main() {
    // Rust test: insert_search_and_delete_work.
    {
        Tree t = Tree::empty()
                     .insert(8).insert(3).insert(10).insert(1)
                     .insert(6).insert(14).insert(4).insert(7);
        ISO_CHECK(t.contains(6));
        ISO_CHECK(t.min_value().value() == 1);
        ISO_CHECK(t.max_value().value() == 14);
        ISO_CHECK(t.kth_smallest(4).value() == 6);
        ISO_CHECK(t.is_valid_rb());
        ISO_CHECK(t.root() != nullptr && t.root()->color == Color::Black);

        Tree d = t.erase(3);
        ISO_CHECK(!d.contains(3));
        ISO_CHECK(d.is_valid_rb());
        ISO_CHECK(t.contains(3));  // original untouched
        ISO_CHECK_EQ_UINT(t.size(), 8u);
        ISO_CHECK_EQ_UINT(d.size(), 7u);
    }

    // Rust test: black_height_and_sorted_output_work.
    {
        Tree t = Tree::empty().insert(2).insert(1).insert(3);
        ISO_CHECK(t.black_height() >= 1u);
        std::vector<int> expected = {1, 2, 3};
        ISO_CHECK(t.to_sorted_array() == expected);
    }

    // Empty-tree edge cases.
    {
        Tree e;
        ISO_CHECK_EQ_UINT(e.size(), 0u);
        ISO_CHECK_EQ_UINT(e.black_height(), 0u);
        ISO_CHECK(!e.min_value().has_value());
        ISO_CHECK(!e.kth_smallest(1).has_value());
        ISO_CHECK(e.find(5) == nullptr);
        ISO_CHECK(e.is_valid_rb());
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
        ISO_CHECK(t.predecessor(5).value() == 4);
        ISO_CHECK(t.successor(5).value() == 6);
    }

    // Delete every element one at a time, verifying the invariant each step.
    {
        int vs[] = {50, 30, 70, 20, 40, 60, 80, 35, 45, 10, 90, 25};
        int order[] = {70, 20, 50, 90, 30, 10, 80, 40, 60, 25, 45, 35};
        Tree t;
        for (int v : vs) {
            t = t.insert(v);
        }
        ISO_CHECK_EQ_UINT(t.size(), 12u);
        ISO_CHECK(t.is_valid_rb());
        std::size_t remaining = 12;
        for (int v : order) {
            t = t.erase(v);
            --remaining;
            ISO_CHECK(!t.contains(v));
            ISO_CHECK(t.is_valid_rb());
            ISO_CHECK_EQ_UINT(t.size(), remaining);
        }
        ISO_CHECK_EQ_UINT(t.size(), 0u);
    }

    // Duplicate insert keeps set semantics.
    {
        Tree t = Tree::empty().insert(5).insert(5).insert(5);
        ISO_CHECK_EQ_UINT(t.size(), 1u);
        ISO_CHECK(t.is_valid_rb());
    }

    // Value-semantics copy is independent of its source.
    {
        Tree a = Tree::empty().insert(1).insert(2).insert(3);
        Tree b = a;
        Tree c = b.insert(4);
        ISO_CHECK_EQ_UINT(a.size(), 3u);
        ISO_CHECK_EQ_UINT(b.size(), 3u);
        ISO_CHECK_EQ_UINT(c.size(), 4u);
        ISO_CHECK(!b.contains(4));
        ISO_CHECK(c.contains(4));
    }

    // Larger stress: insert 0..199 ascending (worst case for a plain BST),
    // confirm balance + order statistics, then delete evens.
    {
        Tree t;
        for (int i = 0; i < 200; ++i) {
            t = t.insert(i);
        }
        ISO_CHECK_EQ_UINT(t.size(), 200u);
        ISO_CHECK(t.is_valid_rb());
        ISO_CHECK(t.kth_smallest(100).value() == 99);
        std::vector<int> sorted = t.to_sorted_array();
        ISO_CHECK_EQ_UINT(sorted.size(), 200u);
        ISO_CHECK_EQ_INT(sorted.front(), 0);
        ISO_CHECK_EQ_INT(sorted.back(), 199);

        for (int i = 0; i < 200; i += 2) {
            t = t.erase(i);
        }
        ISO_CHECK_EQ_UINT(t.size(), 100u);
        ISO_CHECK(t.is_valid_rb());
        ISO_CHECK(!t.contains(100));
        ISO_CHECK(t.contains(101));
    }

    return ISO_TEST_RESULT();
}
