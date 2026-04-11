"""
radix_tree — Radix Tree (Compressed Trie / Patricia Trie).

Public surface:
    RadixTree  — the main generic tree class
    RadixNode  — the generic node class (exposed for type annotations)
"""

from radix_tree.radix_tree import RadixNode, RadixTree

__all__ = ["RadixTree", "RadixNode"]
