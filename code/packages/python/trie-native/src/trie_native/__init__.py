"""
Trie -- Native Extension (Rust-backed via python-bridge)
========================================================

A drop-in native counterpart to the pure Python trie package.
"""

from trie_native.trie_native import (  # type: ignore[import]
    KeyNotFoundError,
    Trie,
    TrieError,
)

__all__ = [
    "Trie",
    "TrieError",
    "KeyNotFoundError",
]
