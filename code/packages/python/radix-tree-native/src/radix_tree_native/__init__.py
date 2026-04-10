"""
Radix tree -- Native Extension (Rust-backed via python-bridge)
==============================================================
"""

from radix_tree_native.radix_tree_native import RadixTree  # type: ignore[import]

__all__ = ["RadixTree"]
