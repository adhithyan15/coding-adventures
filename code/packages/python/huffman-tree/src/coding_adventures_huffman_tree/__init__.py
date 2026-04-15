"""
coding_adventures_huffman_tree — DT27: Huffman Tree
====================================================

A full binary tree that assigns optimal prefix-free bit codes to symbols.
Symbols with higher frequencies receive shorter codes; symbols with lower
frequencies receive longer codes.  The resulting code is optimal: no other
prefix-free code can achieve a lower expected bit-length for the given symbol
distribution.

**Layer position**::

    DT04: heap            ← used during construction
    DT27: huffman-tree    ← [YOU ARE HERE]

Public API::

    from coding_adventures_huffman_tree import HuffmanTree
"""

from coding_adventures_huffman_tree.huffman_tree import HuffmanTree

__version__ = "0.1.0"

__all__ = ["HuffmanTree"]
