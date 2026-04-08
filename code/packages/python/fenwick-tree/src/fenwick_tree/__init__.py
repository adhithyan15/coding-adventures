"""
fenwick_tree — Binary Indexed Tree (Fenwick Tree).

Public surface:
    FenwickTree   — the main class
    FenwickError  — base exception
"""

from fenwick_tree.fenwick_tree import FenwickError, FenwickTree

__all__ = ["FenwickTree", "FenwickError"]
