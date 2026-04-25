from avl_tree import AVLNode, AVLTree


def test_rotations_rebalance_the_tree() -> None:
    tree = AVLTree.from_values([10, 20, 30])

    assert tree.to_sorted_array() == [10, 20, 30]
    assert tree.root.value == 20  # type: ignore[union-attr]
    assert tree.is_valid_bst()
    assert tree.is_valid_avl()
    assert tree.height() == 1
    assert tree.size() == 3

    tree = AVLTree.from_values([30, 20, 10])
    assert tree.root.value == 20  # type: ignore[union-attr]
    assert tree.is_valid_avl()


def test_search_and_order_statistics_work() -> None:
    tree = AVLTree.from_values([40, 20, 60, 10, 30, 50, 70])

    assert tree.search(20).value == 20  # type: ignore[union-attr]
    assert tree.contains(50)
    assert tree.min_value() == 10
    assert tree.max_value() == 70
    assert tree.predecessor(40) == 30
    assert tree.successor(40) == 50
    assert tree.kth_smallest(4) == 40
    assert tree.rank(35) == 3

    deleted = tree.delete(20)
    assert not deleted.contains(20)
    assert deleted.is_valid_avl()
    assert tree.contains(20)


def test_edge_cases_and_duplicates() -> None:
    tree: AVLTree[int] = AVLTree.empty()

    assert tree.search(1) is None
    assert tree.min_value() is None
    assert tree.max_value() is None
    assert tree.predecessor(1) is None
    assert tree.successor(1) is None
    assert tree.kth_smallest(0) is None
    assert tree.rank(1) == 0
    assert tree.balance_factor(None) == 0

    tree = AVLTree.from_values([30, 20, 40, 10, 25, 35, 50])
    duplicate = tree.insert(25)
    assert duplicate.to_sorted_array() == tree.to_sorted_array()
    assert tree.delete(999).to_sorted_array() == tree.to_sorted_array()

    single = AVLTree(5)
    assert single.root.value == 5  # type: ignore[union-attr]
    assert single.height() == 0
    assert AVLTree.from_values([1, 2]).delete(1).to_sorted_array() == [2]
    assert AVLTree.from_values([2, 1]).delete(2).to_sorted_array() == [1]


def test_double_rotations_and_validation_failures() -> None:
    left_right = AVLTree.from_values([30, 10, 20])
    right_left = AVLTree.from_values([10, 30, 20])

    assert left_right.root.value == 20  # type: ignore[union-attr]
    assert right_left.root.value == 20  # type: ignore[union-attr]
    assert left_right.is_valid_avl()
    assert right_left.is_valid_avl()

    bad_order = AVLTree(AVLNode(5, left=AVLNode(6), height=1, size=2))
    bad_right_order = AVLTree(AVLNode(5, right=AVLNode(4), height=1, size=2))
    bad_height = AVLTree(AVLNode(5, left=AVLNode(3), height=99, size=2))
    assert not bad_order.is_valid_bst()
    assert not bad_order.is_valid_avl()
    assert not bad_right_order.is_valid_bst()
    assert not bad_right_order.is_valid_avl()
    assert not bad_height.is_valid_avl()


def test_delete_with_nested_successor_and_order_branches() -> None:
    tree = AVLTree.from_values([5, 3, 8, 7, 9, 6])

    deleted = tree.delete(5)
    assert deleted.to_sorted_array() == [3, 6, 7, 8, 9]
    assert deleted.is_valid_avl()
    assert tree.kth_smallest(1) == 3
    assert tree.kth_smallest(6) == 9
    assert tree.rank(5) == 1
