"""Functional binary search tree with order statistics."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Generic, Iterable, TypeVar

T = TypeVar("T")
_UNSET = object()


@dataclass(slots=True)
class BSTNode(Generic[T]):
    value: T
    left: BSTNode[T] | None = None
    right: BSTNode[T] | None = None
    size: int = 1


class BinarySearchTree(Generic[T]):
    def __init__(self, root: BSTNode[T] | T | None = None) -> None:
        if root is None or isinstance(root, BSTNode):
            self.root = root
        else:
            self.root = BSTNode(root)

    @classmethod
    def empty(cls) -> BinarySearchTree[T]:
        return cls()

    @classmethod
    def from_sorted_array(cls, values: Iterable[T]) -> BinarySearchTree[T]:
        return cls(_build_balanced(list(values)))

    def insert(self, value: T) -> BinarySearchTree[T]:
        return BinarySearchTree(_insert(self.root, value))

    def delete(self, value: T) -> BinarySearchTree[T]:
        return BinarySearchTree(_delete(self.root, value))

    def search(self, value: T) -> BSTNode[T] | None:
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
        current = self.root
        while current is not None and current.left is not None:
            current = current.left
        return None if current is None else current.value

    def max_value(self) -> T | None:
        current = self.root
        while current is not None and current.right is not None:
            current = current.right
        return None if current is None else current.value

    def predecessor(self, value: T) -> T | None:
        current = self.root
        best: T | None = None
        while current is not None:
            if value <= current.value:  # type: ignore[operator]
                current = current.left
            else:
                best = current.value
                current = current.right
        return best

    def successor(self, value: T) -> T | None:
        current = self.root
        best: T | None = None
        while current is not None:
            if value >= current.value:  # type: ignore[operator]
                current = current.right
            else:
                best = current.value
                current = current.left
        return best

    def kth_smallest(self, k: int) -> T | None:
        return _kth_smallest(self.root, k)

    def rank(self, value: T) -> int:
        return _rank(self.root, value)

    def to_sorted_array(self) -> list[T]:
        out: list[T] = []
        _inorder(self.root, out)
        return out

    def is_valid(self) -> bool:
        return _validate(self.root, None, None) is not None

    def height(self) -> int:
        return _height(self.root)

    def size(self) -> int:
        return _size(self.root)

    def __repr__(self) -> str:
        root = None if self.root is None else self.root.value
        return f"BinarySearchTree(root={root!r}, size={self.size()})"


def _insert(root: BSTNode[T] | None, value: T) -> BSTNode[T]:
    if root is None:
        return BSTNode(value)

    if value < root.value:  # type: ignore[operator]
        return _with_children(root, left=_insert(root.left, value))
    if value > root.value:  # type: ignore[operator]
        return _with_children(root, right=_insert(root.right, value))
    return root


def _delete(root: BSTNode[T] | None, value: T) -> BSTNode[T] | None:
    if root is None:
        return None

    if value < root.value:  # type: ignore[operator]
        return _with_children(root, left=_delete(root.left, value))
    if value > root.value:  # type: ignore[operator]
        return _with_children(root, right=_delete(root.right, value))

    if root.left is None:
        return root.right
    if root.right is None:
        return root.left

    new_right, successor = _extract_min(root.right)
    return _node(successor, root.left, new_right)


def _extract_min(root: BSTNode[T]) -> tuple[BSTNode[T] | None, T]:
    if root.left is None:
        return root.right, root.value
    new_left, minimum = _extract_min(root.left)
    return _with_children(root, left=new_left), minimum


def _kth_smallest(root: BSTNode[T] | None, k: int) -> T | None:
    if root is None or k <= 0:
        return None
    left_size = _size(root.left)
    if k == left_size + 1:
        return root.value
    if k <= left_size:
        return _kth_smallest(root.left, k)
    return _kth_smallest(root.right, k - left_size - 1)


def _rank(root: BSTNode[T] | None, value: T) -> int:
    if root is None:
        return 0
    if value < root.value:  # type: ignore[operator]
        return _rank(root.left, value)
    if value > root.value:  # type: ignore[operator]
        return _size(root.left) + 1 + _rank(root.right, value)
    return _size(root.left)


def _inorder(root: BSTNode[T] | None, out: list[T]) -> None:
    if root is None:
        return
    _inorder(root.left, out)
    out.append(root.value)
    _inorder(root.right, out)


def _validate(
    root: BSTNode[T] | None,
    minimum: T | None,
    maximum: T | None,
) -> tuple[int, int] | None:
    if root is None:
        return -1, 0
    if minimum is not None and root.value <= minimum:  # type: ignore[operator]
        return None
    if maximum is not None and root.value >= maximum:  # type: ignore[operator]
        return None

    left = _validate(root.left, minimum, root.value)
    right = _validate(root.right, root.value, maximum)
    if left is None or right is None:
        return None
    height = 1 + max(left[0], right[0])
    size = 1 + left[1] + right[1]
    if root.size != size:
        return None
    return height, size


def _height(root: BSTNode[T] | None) -> int:
    if root is None:
        return -1
    return 1 + max(_height(root.left), _height(root.right))


def _size(root: BSTNode[T] | None) -> int:
    return 0 if root is None else root.size


def _build_balanced(values: list[T]) -> BSTNode[T] | None:
    if not values:
        return None
    mid = len(values) // 2
    return _node(
        values[mid],
        _build_balanced(values[:mid]),
        _build_balanced(values[mid + 1 :]),
    )


def _with_children(
    root: BSTNode[T],
    *,
    left: BSTNode[T] | None | object = _UNSET,
    right: BSTNode[T] | None | object = _UNSET,
) -> BSTNode[T]:
    return _node(
        root.value,
        root.left if left is _UNSET else left,  # type: ignore[arg-type]
        root.right if right is _UNSET else right,  # type: ignore[arg-type]
    )


def _node(value: T, left: BSTNode[T] | None, right: BSTNode[T] | None) -> BSTNode[T]:
    return BSTNode(value, left=left, right=right, size=1 + _size(left) + _size(right))
