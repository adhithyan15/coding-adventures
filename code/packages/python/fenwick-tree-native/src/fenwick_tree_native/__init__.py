"""
Fenwick tree -- Native Extension (Rust-backed via python-bridge)
================================================================
"""

from fenwick_tree_native.fenwick_tree_native import (  # type: ignore[import]
    EmptyTreeError,
    FenwickError,
    FenwickTree,
    IndexOutOfRangeError,
)

__all__ = [
    "FenwickTree",
    "FenwickError",
    "IndexOutOfRangeError",
    "EmptyTreeError",
]
