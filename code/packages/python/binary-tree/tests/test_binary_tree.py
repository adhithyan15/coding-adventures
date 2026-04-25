from binary_tree import BinaryTree, BinaryTreeNode


def test_level_order_round_trip() -> None:
    tree = BinaryTree.from_level_order([1, 2, 3, 4, 5, 6, 7])

    assert tree.root is not None
    assert tree.root.value == 1
    assert tree.to_array() == [1, 2, 3, 4, 5, 6, 7]
    assert tree.level_order() == [1, 2, 3, 4, 5, 6, 7]


def test_shape_queries_work() -> None:
    tree = BinaryTree.from_level_order([1, 2, None])

    assert not tree.is_full()
    assert tree.is_complete()
    assert not tree.is_perfect()
    assert tree.height() == 1
    assert tree.size() == 2
    assert tree.left_child(1).value == 2  # type: ignore[union-attr]
    assert tree.right_child(1) is None
    assert tree.find(999) is None


def test_traversals_work() -> None:
    tree = BinaryTree.from_level_order([1, 2, 3, 4, None, 5, None])

    assert tree.preorder() == [1, 2, 4, 3, 5]
    assert tree.inorder() == [4, 2, 1, 5, 3]
    assert tree.postorder() == [4, 2, 5, 3, 1]
    assert tree.level_order() == [1, 2, 3, 4, 5]
    assert tree.to_array() == [1, 2, 3, 4, None, 5, None]


def test_full_complete_and_perfect_tree() -> None:
    tree = BinaryTree.from_level_order(["A", "B", "C", "D", "E", "F", "G"])

    assert tree.is_full()
    assert tree.is_complete()
    assert tree.is_perfect()
    assert tree.left_child("A").value == "B"  # type: ignore[union-attr]
    assert tree.right_child("A").value == "C"  # type: ignore[union-attr]


def test_empty_tree() -> None:
    tree: BinaryTree[int] = BinaryTree()

    assert tree.root is None
    assert tree.is_full()
    assert tree.is_complete()
    assert tree.is_perfect()
    assert tree.height() == -1
    assert tree.size() == 0
    assert tree.to_array() == []
    assert tree.to_ascii() == ""
    assert tree.level_order() == []
    assert repr(tree) == "BinaryTree(root=None, size=0)"


def test_with_root_and_ascii_render() -> None:
    root = BinaryTreeNode(
        "root",
        left=BinaryTreeNode("left"),
        right=BinaryTreeNode("right"),
    )
    tree = BinaryTree.with_root(root)

    ascii_tree = tree.to_ascii()
    assert "root" in ascii_tree
    assert "left" in ascii_tree
    assert "right" in ascii_tree
    assert repr(tree) == "BinaryTree(root='root', size=3)"
