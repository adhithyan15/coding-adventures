from __future__ import annotations

from tree_set_native import TreeSet


def test_insert_lookup_and_iteration() -> None:
    tree = TreeSet([5, 1, 3, 3, 9])
    tree.add(7)

    assert len(tree) == 5
    assert tree.size() == 5
    assert tree.contains(7)
    assert list(tree) == [1, 3, 5, 7, 9]
    assert tree.to_sorted_array() == [1, 3, 5, 7, 9]


def test_rank_range_and_selection() -> None:
    tree = TreeSet([10, 20, 30, 40])

    assert tree.rank(25) == 2
    assert tree.by_rank(0) == 10
    assert tree.kth_smallest(3) == 30
    assert tree.predecessor(30) == 20
    assert tree.successor(30) == 40
    assert tree.range(15, 35) == [20, 30]


def test_set_algebra() -> None:
    left = TreeSet([1, 2, 3, 5])
    right = TreeSet([3, 4, 5, 6])

    assert left.union(right).to_sorted_array() == [1, 2, 3, 4, 5, 6]
    assert left.intersection(right).to_sorted_array() == [3, 5]
    assert left.difference(right).to_sorted_array() == [1, 2]
    assert left.symmetric_difference(right).to_sorted_array() == [1, 2, 4, 6]
    assert left.is_subset(left.union(right))
    assert left.is_superset(left.intersection(right))
    assert left.is_disjoint(TreeSet([8, 9]))
    assert left.equals(TreeSet([1, 2, 3, 5]))
