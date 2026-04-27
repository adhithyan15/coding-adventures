"""Persistent AVL tree with order statistics."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Generic, Iterable, TypeVar

T = TypeVar("T")


@dataclass(slots=True)
class AVLNode(Generic[T]):
    value: T
    left: AVLNode[T] | None = None
    right: AVLNode[T] | None = None
    height: int = 0
    size: int = 1


class AVLTree(Generic[T]):
    def __init__(self, root: AVLNode[T] | T | None = None) -> None:
        if root is None or isinstance(root, AVLNode):
            self.root = root
        else:
            self.root = AVLNode(root)

    @classmethod
    def empty(cls) -> AVLTree[T]:
        return cls()

    @classmethod
    def from_values(cls, values: Iterable[T]) -> AVLTree[T]:
        tree = cls.empty()
        for value in values:
            tree = tree.insert(value)
        return tree

    def insert(self, value: T) -> AVLTree[T]:
        return AVLTree(_insert(self.root, value))

    def delete(self, value: T) -> AVLTree[T]:
        return AVLTree(_delete(self.root, value))

    def search(self, value: T) -> AVLNode[T] | None:
        current = self.root
        while current is not None:
            if value < current.value:  # type: ignore[operator]
                current = current.left
            elif value > current.value:  # type: ignore[operator]
                current = current.right
            else:
                return current
        return None

    def contains(self, value: T) -> bool:
        return self.search(value) is not None

    def min_value(self) -> T | None:
        node = self.root
        while node is not None and node.left is not None:
            node = node.left
        return None if node is None else node.value

    def max_value(self) -> T | None:
        node = self.root
        while node is not None and node.right is not None:
            node = node.right
        return None if node is None else node.value

    def predecessor(self, value: T) -> T | None:
        node = self.root
        best: T | None = None
        while node is not None:
            if value <= node.value:  # type: ignore[operator]
                node = node.left
            else:
                best = node.value
                node = node.right
        return best

    def successor(self, value: T) -> T | None:
        node = self.root
        best: T | None = None
        while node is not None:
            if value >= node.value:  # type: ignore[operator]
                node = node.right
            else:
                best = node.value
                node = node.left
        return best

    def kth_smallest(self, k: int) -> T | None:
        return _kth(self.root, k)

    def rank(self, value: T) -> int:
        return _rank(self.root, value)

    def to_sorted_array(self) -> list[T]:
        out: list[T] = []
        _inorder(self.root, out)
        return out

    def is_valid_bst(self) -> bool:
        return _validate_bst(self.root, None, None)

    def is_valid_avl(self) -> bool:
        return _validate_avl(self.root, None, None) is not None

    def balance_factor(self, node: AVLNode[T] | None) -> int:
        return _height(node.left) - _height(node.right) if node is not None else 0

    def height(self) -> int:
        return _height(self.root)

    def size(self) -> int:
        return _size(self.root)


def _insert(root: AVLNode[T] | None, value: T) -> AVLNode[T]:
    if root is None:
        return AVLNode(value)
    if value < root.value:  # type: ignore[operator]
        return _rebalance(_node(root.value, _insert(root.left, value), root.right))
    if value > root.value:  # type: ignore[operator]
        return _rebalance(_node(root.value, root.left, _insert(root.right, value)))
    return root


def _delete(root: AVLNode[T] | None, value: T) -> AVLNode[T] | None:
    if root is None:
        return None
    if value < root.value:  # type: ignore[operator]
        return _rebalance(_node(root.value, _delete(root.left, value), root.right))
    if value > root.value:  # type: ignore[operator]
        return _rebalance(_node(root.value, root.left, _delete(root.right, value)))
    if root.left is None:
        return root.right
    if root.right is None:
        return root.left
    new_right, successor = _extract_min(root.right)
    return _rebalance(_node(successor, root.left, new_right))


def _extract_min(root: AVLNode[T]) -> tuple[AVLNode[T] | None, T]:
    if root.left is None:
        return root.right, root.value
    new_left, minimum = _extract_min(root.left)
    return _rebalance(_node(root.value, new_left, root.right)), minimum


def _rebalance(node: AVLNode[T]) -> AVLNode[T]:
    bf = _balance_factor(node)
    if bf > 1:
        left = node.left
        if left is not None and _balance_factor(left) < 0:
            left = _rotate_left(left)
        return _rotate_right(_node(node.value, left, node.right))
    if bf < -1:
        right = node.right
        if right is not None and _balance_factor(right) > 0:
            right = _rotate_right(right)
        return _rotate_left(_node(node.value, node.left, right))
    return node


def _rotate_left(root: AVLNode[T]) -> AVLNode[T]:
    right = root.right
    if right is None:
        return root
    new_left = _node(root.value, root.left, right.left)
    return _node(right.value, new_left, right.right)


def _rotate_right(root: AVLNode[T]) -> AVLNode[T]:
    left = root.left
    if left is None:
        return root
    new_right = _node(root.value, left.right, root.right)
    return _node(left.value, left.left, new_right)


def _balance_factor(node: AVLNode[T]) -> int:
    return _height(node.left) - _height(node.right)


def _kth(root: AVLNode[T] | None, k: int) -> T | None:
    if root is None or k <= 0:
        return None
    left_size = _size(root.left)
    if k == left_size + 1:
        return root.value
    if k <= left_size:
        return _kth(root.left, k)
    return _kth(root.right, k - left_size - 1)


def _rank(root: AVLNode[T] | None, value: T) -> int:
    if root is None:
        return 0
    if value < root.value:  # type: ignore[operator]
        return _rank(root.left, value)
    if value > root.value:  # type: ignore[operator]
        return _size(root.left) + 1 + _rank(root.right, value)
    return _size(root.left)


def _inorder(root: AVLNode[T] | None, out: list[T]) -> None:
    if root is None:
        return
    _inorder(root.left, out)
    out.append(root.value)
    _inorder(root.right, out)


def _validate_bst(root: AVLNode[T] | None, minimum: T | None, maximum: T | None) -> bool:
    if root is None:
        return True
    if minimum is not None and root.value <= minimum:  # type: ignore[operator]
        return False
    if maximum is not None and root.value >= maximum:  # type: ignore[operator]
        return False
    return _validate_bst(root.left, minimum, root.value) and _validate_bst(root.right, root.value, maximum)


def _validate_avl(
    root: AVLNode[T] | None,
    minimum: T | None,
    maximum: T | None,
) -> tuple[int, int] | None:
    if root is None:
        return -1, 0
    if minimum is not None and root.value <= minimum:  # type: ignore[operator]
        return None
    if maximum is not None and root.value >= maximum:  # type: ignore[operator]
        return None
    left = _validate_avl(root.left, minimum, root.value)
    right = _validate_avl(root.right, root.value, maximum)
    if left is None or right is None:
        return None
    height = 1 + max(left[0], right[0])
    size = 1 + left[1] + right[1]
    if root.height != height or root.size != size or abs(left[0] - right[0]) > 1:
        return None
    return height, size


def _height(root: AVLNode[T] | None) -> int:
    return -1 if root is None else root.height


def _size(root: AVLNode[T] | None) -> int:
    return 0 if root is None else root.size


def _node(value: T, left: AVLNode[T] | None, right: AVLNode[T] | None) -> AVLNode[T]:
    return AVLNode(value, left, right, 1 + max(_height(left), _height(right)), 1 + _size(left) + _size(right))
