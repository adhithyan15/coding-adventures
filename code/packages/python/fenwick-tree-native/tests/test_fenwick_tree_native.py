from __future__ import annotations

import pytest

from fenwick_tree_native import (
    EmptyTreeError,
    FenwickError,
    FenwickTree,
    IndexOutOfRangeError,
)


def test_from_list_and_queries() -> None:
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    assert len(tree) == 5
    assert tree.prefix_sum(3) == 6.0
    assert tree.range_sum(2, 4) == 10.0
    assert tree.point_query(4) == 7.0


def test_updates_change_subsequent_queries() -> None:
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    tree.update(3, 5)
    assert tree.point_query(3) == 6.0
    assert tree.prefix_sum(4) == 18.0


def test_find_kth_examples() -> None:
    tree = FenwickTree.from_list([1, 2, 3, 4, 5])
    assert tree.find_kth(1) == 1
    assert tree.find_kth(3) == 2
    assert tree.find_kth(10) == 4


def test_error_mapping() -> None:
    tree = FenwickTree.from_list([1, 2, 3])
    with pytest.raises(IndexOutOfRangeError):
        tree.prefix_sum(4)
    with pytest.raises(FenwickError):
        tree.range_sum(3, 1)
    with pytest.raises(FenwickError):
        tree.find_kth(0)
    with pytest.raises(EmptyTreeError):
        FenwickTree(0).find_kth(1)
