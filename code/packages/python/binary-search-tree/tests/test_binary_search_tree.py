from binary_search_tree import BSTNode, BinarySearchTree


def populated() -> BinarySearchTree[int]:
    tree = BinarySearchTree.empty()
    for value in [5, 1, 8, 3, 7]:
        tree = tree.insert(value)
    return tree


def test_insert_search_and_delete_work() -> None:
    tree = populated()

    assert tree.to_sorted_array() == [1, 3, 5, 7, 8]
    assert tree.size() == 5
    assert tree.contains(7)
    assert tree.search(7).value == 7  # type: ignore[union-attr]
    assert tree.min_value() == 1
    assert tree.max_value() == 8
    assert tree.predecessor(5) == 3
    assert tree.successor(5) == 7
    assert tree.rank(4) == 2
    assert tree.kth_smallest(4) == 7

    deleted = tree.delete(5)
    assert not deleted.contains(5)
    assert deleted.is_valid()
    assert tree.contains(5)


def test_from_sorted_array_builds_balanced_tree() -> None:
    tree = BinarySearchTree.from_sorted_array([1, 2, 3, 4, 5, 6, 7])

    assert tree.to_sorted_array() == [1, 2, 3, 4, 5, 6, 7]
    assert tree.height() == 2
    assert tree.size() == 7
    assert tree.is_valid()


def test_empty_and_edge_cases() -> None:
    tree: BinarySearchTree[int] = BinarySearchTree.empty()

    assert tree.search(1) is None
    assert tree.min_value() is None
    assert tree.max_value() is None
    assert tree.predecessor(1) is None
    assert tree.successor(1) is None
    assert tree.kth_smallest(0) is None
    assert tree.kth_smallest(1) is None
    assert tree.rank(1) == 0
    assert tree.height() == -1
    assert tree.size() == 0
    assert repr(tree) == "BinarySearchTree(root=None, size=0)"


def test_duplicate_and_single_child_delete() -> None:
    tree = BinarySearchTree.from_sorted_array([2, 4, 6, 8])

    assert tree.root.value == 6  # type: ignore[union-attr]
    duplicate = tree.insert(4)
    assert duplicate.to_sorted_array() == tree.to_sorted_array()
    assert tree.delete(2).to_sorted_array() == [4, 6, 8]


def test_validation_catches_bad_shape_and_size() -> None:
    bad_order = BinarySearchTree(BSTNode(5, left=BSTNode(6)))
    bad_size = BinarySearchTree(BSTNode(5, left=BSTNode(3), size=99))

    assert not bad_order.is_valid()
    assert not bad_size.is_valid()
