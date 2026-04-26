"""Generic binary tree utilities matching the Rust reference package."""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from typing import Generic, Iterable, TypeVar

T = TypeVar("T")


@dataclass(slots=True)
class BinaryTreeNode(Generic[T]):
    """A node in a binary tree."""

    value: T
    left: BinaryTreeNode[T] | None = None
    right: BinaryTreeNode[T] | None = None


class BinaryTree(Generic[T]):
    """A generic binary tree with traversal and shape helpers."""

    def __init__(self, root: BinaryTreeNode[T] | T | None = None) -> None:
        if root is None or isinstance(root, BinaryTreeNode):
            self.root = root
        else:
            self.root = BinaryTreeNode(root)

    @classmethod
    def with_root(cls, root: BinaryTreeNode[T] | None) -> BinaryTree[T]:
        return cls(root)

    @classmethod
    def from_level_order(cls, values: Iterable[T | None]) -> BinaryTree[T]:
        items = list(values)
        return cls(_build_from_level_order(items, 0))

    def find(self, value: T) -> BinaryTreeNode[T] | None:
        return find(self.root, value)

    def left_child(self, value: T) -> BinaryTreeNode[T] | None:
        node = self.find(value)
        return None if node is None else node.left

    def right_child(self, value: T) -> BinaryTreeNode[T] | None:
        node = self.find(value)
        return None if node is None else node.right

    def is_full(self) -> bool:
        return is_full(self.root)

    def is_complete(self) -> bool:
        return is_complete(self.root)

    def is_perfect(self) -> bool:
        return is_perfect(self.root)

    def height(self) -> int:
        return height(self.root)

    def size(self) -> int:
        return size(self.root)

    def inorder(self) -> list[T]:
        out: list[T] = []
        _inorder(self.root, out)
        return out

    def preorder(self) -> list[T]:
        out: list[T] = []
        _preorder(self.root, out)
        return out

    def postorder(self) -> list[T]:
        out: list[T] = []
        _postorder(self.root, out)
        return out

    def level_order(self) -> list[T]:
        if self.root is None:
            return []

        out: list[T] = []
        queue: deque[BinaryTreeNode[T]] = deque([self.root])
        while queue:
            node = queue.popleft()
            out.append(node.value)
            if node.left is not None:
                queue.append(node.left)
            if node.right is not None:
                queue.append(node.right)
        return out

    def to_array(self) -> list[T | None]:
        tree_height = self.height()
        if tree_height < 0:
            return []

        values: list[T | None] = [None] * ((1 << (tree_height + 1)) - 1)
        _fill_array(self.root, 0, values)
        return values

    def to_ascii(self) -> str:
        if self.root is None:
            return ""

        lines: list[str] = []
        _render_ascii(self.root, "", True, lines)
        return "\n".join(lines)

    def __repr__(self) -> str:
        value = None if self.root is None else self.root.value
        return f"BinaryTree(root={value!r}, size={self.size()})"


def find(root: BinaryTreeNode[T] | None, value: T) -> BinaryTreeNode[T] | None:
    if root is None:
        return None
    if root.value == value:
        return root
    return find(root.left, value) or find(root.right, value)


def is_full(root: BinaryTreeNode[T] | None) -> bool:
    if root is None:
        return True
    if root.left is None and root.right is None:
        return True
    if root.left is None or root.right is None:
        return False
    return is_full(root.left) and is_full(root.right)


def is_complete(root: BinaryTreeNode[T] | None) -> bool:
    queue: deque[BinaryTreeNode[T] | None] = deque([root])
    seen_none = False

    while queue:
        node = queue.popleft()
        if node is None:
            seen_none = True
            continue
        if seen_none:
            return False
        queue.append(node.left)
        queue.append(node.right)

    return True


def is_perfect(root: BinaryTreeNode[T] | None) -> bool:
    tree_height = height(root)
    if tree_height < 0:
        return size(root) == 0
    return size(root) == (1 << (tree_height + 1)) - 1


def height(root: BinaryTreeNode[T] | None) -> int:
    if root is None:
        return -1
    return 1 + max(height(root.left), height(root.right))


def size(root: BinaryTreeNode[T] | None) -> int:
    if root is None:
        return 0
    return 1 + size(root.left) + size(root.right)


def _build_from_level_order(values: list[T | None], index: int) -> BinaryTreeNode[T] | None:
    if index >= len(values):
        return None
    value = values[index]
    if value is None:
        return None
    return BinaryTreeNode(
        value=value,
        left=_build_from_level_order(values, 2 * index + 1),
        right=_build_from_level_order(values, 2 * index + 2),
    )


def _inorder(root: BinaryTreeNode[T] | None, out: list[T]) -> None:
    if root is None:
        return
    _inorder(root.left, out)
    out.append(root.value)
    _inorder(root.right, out)


def _preorder(root: BinaryTreeNode[T] | None, out: list[T]) -> None:
    if root is None:
        return
    out.append(root.value)
    _preorder(root.left, out)
    _preorder(root.right, out)


def _postorder(root: BinaryTreeNode[T] | None, out: list[T]) -> None:
    if root is None:
        return
    _postorder(root.left, out)
    _postorder(root.right, out)
    out.append(root.value)


def _fill_array(root: BinaryTreeNode[T] | None, index: int, out: list[T | None]) -> None:
    if root is None or index >= len(out):
        return
    out[index] = root.value
    _fill_array(root.left, 2 * index + 1, out)
    _fill_array(root.right, 2 * index + 2, out)


def _render_ascii(
    node: BinaryTreeNode[T],
    prefix: str,
    is_tail: bool,
    lines: list[str],
) -> None:
    connector = "`-- " if is_tail else "|-- "
    lines.append(f"{prefix}{connector}{node.value!r}")

    children = [child for child in (node.left, node.right) if child is not None]
    next_prefix = f"{prefix}{'    ' if is_tail else '|   '}"
    for index, child in enumerate(children):
        _render_ascii(child, next_prefix, index + 1 == len(children), lines)
