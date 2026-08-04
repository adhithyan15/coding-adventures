// Tests for the C++ tree-set, using the iso_test.h harness. Mirrors the Rust
// crate's unit tests (ordered-set operations + set algebra) on the default AVL
// backend, and adds persistence and relation checks.
#include "iso_test.h"

#include <vector>

#include "red_black_tree.hpp"  // second backend, to exercise genericity
#include "tree_set.hpp"

using Set = ca::tree_set::TreeSet<int>;

int main() {
    // Rust test: avl_backend_supports_ordered_set_operations.
    {
        Set set = Set::from_list({7, 3, 9, 1, 5, 3});  // dup 3 collapses
        std::vector<int> expected = {1, 3, 5, 7, 9};
        ISO_CHECK(set.to_sorted_array() == expected);
        ISO_CHECK_EQ_UINT(set.size(), 5u);
        ISO_CHECK(set.min_value().value() == 1);
        ISO_CHECK(set.max_value().value() == 9);
        ISO_CHECK_EQ_UINT(set.rank(7), 3u);
        ISO_CHECK(set.kth_smallest(3).value() == 5);

        std::vector<int> r_incl = {3, 5, 7};
        std::vector<int> r_excl = {5};
        ISO_CHECK(set.range(3, 7, true) == r_incl);
        ISO_CHECK(set.range(3, 7, false) == r_excl);
        ISO_CHECK(set.backend().is_valid_avl());

        Set removed = set.remove(5);  // persistent
        std::vector<int> after = {1, 3, 7, 9};
        ISO_CHECK(removed.to_sorted_array() == after);
        ISO_CHECK(set.to_sorted_array() == expected);  // original untouched
    }

    // Rust test: avl_backend_set_algebra_works.
    {
        Set left = Set::from_list({1, 2, 3, 5});
        Set right = Set::from_list({3, 4, 5, 6});

        std::vector<int> eu = {1, 2, 3, 4, 5, 6};
        std::vector<int> ei = {3, 5};
        std::vector<int> ed = {1, 2};
        std::vector<int> es = {1, 2, 4, 6};
        ISO_CHECK(left.union_with(right).to_sorted_array() == eu);
        ISO_CHECK(left.intersection(right).to_sorted_array() == ei);
        ISO_CHECK(left.difference(right).to_sorted_array() == ed);
        ISO_CHECK(left.symmetric_difference(right).to_sorted_array() == es);

        ISO_CHECK(left.is_subset(left.union_with(right)));
        ISO_CHECK(left.is_superset(
            left.intersection(right).union_with(Set::from_list({1, 2}))));
        ISO_CHECK(left.is_disjoint(Set::from_list({8, 9})));
        ISO_CHECK(!left.is_disjoint(right));
        ISO_CHECK(left.equals(Set::from_list({1, 2, 3, 5})));
        ISO_CHECK(!left.equals(right));
    }

    // Rust test: red_black_backend_supports_the_same_api — exercised here on the
    // ca::rb::RBTree backend to prove the template is genuinely backend-generic.
    {
        using RbSet = ca::tree_set::TreeSet<int, ca::rb::RBTree<int>>;
        RbSet set = RbSet::from_list({10, 4, 14, 2, 8, 12, 16});
        std::vector<int> expected = {2, 4, 8, 10, 12, 14, 16};
        ISO_CHECK(set.to_sorted_array() == expected);
        ISO_CHECK(set.backend().is_valid_rb());
        ISO_CHECK(set.predecessor(10).value() == 8);
        ISO_CHECK(set.successor(10).value() == 12);
        ISO_CHECK(set.contains(14));
        std::vector<int> after = {2, 4, 10, 12, 14, 16};
        ISO_CHECK(set.remove(8).to_sorted_array() == after);
    }

    // Edge cases: empty set, range with min > max, empty algebra.
    {
        Set e;
        Set e2;
        ISO_CHECK(e.is_empty());
        ISO_CHECK_EQ_UINT(e.size(), 0u);
        ISO_CHECK(!e.min_value().has_value());
        ISO_CHECK(e.range(1, 10, true).empty());
        ISO_CHECK(e.union_with(e2).is_empty());
        ISO_CHECK(e.is_subset(e2));
        ISO_CHECK(e.is_disjoint(e2));
        ISO_CHECK(e.equals(e2));

        Set s = Set::from_list({5});
        ISO_CHECK(s.range(10, 1, true).empty());  // min > max
    }

    return ISO_TEST_RESULT();
}
