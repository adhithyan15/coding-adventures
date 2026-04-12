"""
trie — Prefix Tree (Trie).

Public surface:
    Trie       — the main generic class (string-keyed, full prefix operations)
    TrieCursor — step-by-step trie cursor for streaming algorithms (generic key type)
    TrieError  — base exception
"""

from trie.trie import Trie, TrieCursor, TrieError

__all__ = ["Trie", "TrieCursor", "TrieError"]
