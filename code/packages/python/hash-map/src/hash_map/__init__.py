"""
hash_map — Hash map with chaining and open-addressing collision strategies.

A hash map (also called a hash table, dictionary, or associative array)
stores key-value pairs and supports O(1) average-case lookup, insertion,
and deletion.

This library implements a ``HashMap[K, V]`` backed by two pluggable
collision strategies:

  - **Chaining** (``"chaining"``): each bucket holds a list of (key, value)
    pairs. Collisions are handled by appending to the list.  Resize when
    load factor exceeds 1.0.

  - **Open addressing** (``"open_addressing"``): a flat array of slots.
    Collisions are resolved by linear probing.  Deleted slots are replaced
    with a TOMBSTONE sentinel so probe chains remain intact.  Resize when
    load factor exceeds 0.75.

Three hash functions are available via the ``hash_fn`` parameter:
  ``"fnv1a"`` (default), ``"murmur3"``, ``"djb2"``.

Quick start::

    >>> from hash_map import HashMap, from_entries, merge
    >>> m = HashMap()
    >>> m.set("hello", 42)
    >>> m.get("hello")
    42
    >>> m.size()
    1
    >>> "hello" in m
    True
"""

from hash_map.hash_map import HashMap, from_entries, merge

__all__ = ["HashMap", "from_entries", "merge"]
