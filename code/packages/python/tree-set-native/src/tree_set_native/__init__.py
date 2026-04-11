"""Tree set native extension."""

from tree_set_native.tree_set_native import TreeSet  # type: ignore[import]


def from_values(values):
    return TreeSet(values)


__all__ = ["TreeSet", "from_values"]
