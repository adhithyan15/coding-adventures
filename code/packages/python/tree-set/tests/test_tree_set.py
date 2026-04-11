from __future__ import annotations

from tree_set import TreeSet, from_values


def test_basic_insertion_lookup_and_iteration() -> None:
    tree = TreeSet([5, 1, 3, 3, 9])
    tree.add(7)

    assert tree.to_sorted_array() == [1, 3, 5, 7, 9]
    assert tree.to_list() == [1, 3, 5, 7, 9]
    assert len(tree) == 5
    assert tree.length == 5
    assert tree.contains(7)
    assert 7 in tree
    assert not tree.contains(2)
    assert list(tree) == [1, 3, 5, 7, 9]


def test_selection_and_rank_helpers() -> None:
    tree = from_values([10, 20, 30, 40])

    assert tree.rank(5) == 0
    assert tree.rank(25) == 2
    assert tree.by_rank(0) == 10
    assert tree.by_rank(3) == 40
    assert tree.kth_smallest(3) == 30
    assert tree.predecessor(30) == 20
    assert tree.successor(30) == 40


def test_range_queries() -> None:
    tree = from_values([1, 3, 5, 7, 9])

    assert tree.range(3, 7) == [3, 5, 7]
    assert tree.range(3, 7, False) == [5]
    assert tree.range(10, 20) == []


def test_set_algebra() -> None:
    left = from_values([1, 2, 3, 5])
    right = from_values([3, 4, 5, 6])

    assert left.union(right).to_sorted_array() == [1, 2, 3, 4, 5, 6]
    assert left.intersection(right).to_sorted_array() == [3, 5]
    assert left.difference(right).to_sorted_array() == [1, 2]
    assert left.symmetric_difference(right).to_sorted_array() == [1, 2, 4, 6]
    assert left.is_subset(left.union(right))
    assert left.is_superset(left.intersection(right))
    assert left.is_disjoint(from_values([8, 9]))
    assert left.equals(from_values([1, 2, 3, 5]))


def test_custom_comparator_and_delete() -> None:
    by_length = TreeSet[str](
        [],
        lambda left, right: (len(left) > len(right)) - (len(left) < len(right))
        if len(left) != len(right)
        else ((left > right) - (left < right)),
    )
    by_length.add("banana")
    by_length.add("fig")
    by_length.add("apple")

    assert by_length.to_sorted_array() == ["fig", "apple", "banana"]
    assert by_length.delete("apple")
    assert by_length.to_sorted_array() == ["fig", "banana"]
    assert not by_length.delete("missing")


def test_empty_set_and_protocol_edge_cases() -> None:
    empty = TreeSet()

    assert empty.is_empty()
    assert empty.isEmpty()
    assert empty.min() is None
    assert empty.max() is None
    assert empty.first() is None
    assert empty.last() is None
    assert empty.kth_smallest(0) is None
    assert empty.to_array() == []
    assert empty.discard(1) is False
    assert empty.delete(1) is False
    assert repr(empty) == "TreeSet([])"
    assert str(empty) == "TreeSet([])"

    class_tree = TreeSet.from_values([4, 2, 8])
    assert class_tree.to_sorted_array() == [2, 4, 8]
    assert class_tree == TreeSet.from_values([2, 4, 8])
    assert not (class_tree == TreeSet.from_values([2, 4, 9]))
    assert not TreeSet.from_values([1, 2]).equals(TreeSet.from_values([1, 2, 3]))
    assert not TreeSet.from_values([1, 3]).is_subset(TreeSet.from_values([2, 3]))
    assert not TreeSet.from_values([1, 2]).is_disjoint(TreeSet.from_values([2, 4]))
    assert TreeSet.from_values([2, 4]).is_disjoint(TreeSet.from_values([1]))
    assert TreeSet.from_values([1, 2]).range(5, 1) == []
    assert (object() in TreeSet.from_values([1])) is False
