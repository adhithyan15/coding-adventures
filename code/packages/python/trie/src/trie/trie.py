"""
trie.py — Prefix Tree (Trie)
=============================

A trie (pronounced "try", from re**trie**val) is a tree where each path from
root to a node spells out a string prefix. Unlike a hash map that treats keys as
opaque, a trie decomposes each key character by character and shares common
prefixes among all words that begin the same way.

=== The Fundamental Idea ===

Imagine the words ["app", "apple", "apply", "apt", "banana"]:

    In a hash map, "app", "apple", "apply" are three separate, unrelated entries.

    In a trie, they share the path root → 'a' → 'p' → 'p':

        (root)
        ├── a
        │   └── p
        │       ├── p (*)   ← "app" ends here
        │       │   └── l
        │       │       ├── e (*)  ← "apple"
        │       │       └── y (*)  ← "apply"
        │       └── t (*)   ← "apt"
        └── b
            └── a → n → a → n → a (*)  ← "banana"

    (*) marks nodes where a complete word ends (is_end = True).

This shared structure makes prefix queries O(p) where p is the prefix length,
vs O(n·k) for a hash map scan. For autocomplete with 100k words returning 50
results, the trie is ~2000× faster.

=== Node Design ===

We use a dict-based node (Design 2 from the spec) rather than a fixed 26-slot
array. This uses less memory, generalises to any character set (Unicode,
DNA bases, etc.), and has O(1) average-case child lookup via Python's hash map.

    class _TrieNode:
        children: dict[str, _TrieNode]   # char → child
        is_end:   bool                   # True if a complete key ends here
        value:    V | None               # stored value (None = no value)

=== Insert ===

Walk the key character by character. If a child for the current character
exists, follow it. If not, create a new node. After the last character,
mark is_end = True and store the value.

    insert("apt"):
        root → 'a' (exists) → 'p' (exists) → 't' (NEW node)
        Mark node_t.is_end = True

    Time: O(len(key))

=== Search ===

Same walk as insert, but don't create nodes. Two conditions must hold:
1. The path for every character in the key exists.
2. The final node has is_end = True.

Condition 2 is what distinguishes `search("app")` (finds value) from
`starts_with("app")` (only checks path exists).

=== Prefix Search (Autocomplete) ===

1. Navigate to the node at the end of the prefix path. O(p).
2. DFS from that node, collecting all words where is_end = True.
   Sort children at each step to guarantee lexicographic output.

    words_with_prefix("app"):
        Navigate: root → 'a' → 'p' → 'p'
        DFS from node_app, accumulating suffix "":
            is_end=True at node_app? → emit "app"
            child 'l' → node_appl:
                child 'e' → node_apple (is_end=True) → emit "apple"
                child 'y' → node_apply (is_end=True) → emit "apply"

    Result: ["app", "apple", "apply"] (sorted automatically by DFS order)

=== Delete ===

After removing a word's is_end flag, walk back up and prune nodes that have
no children and no is_end. This prevents memory from growing unboundedly when
words are inserted and deleted over time.

    delete "apple" from {"app", "apple"}:
        Navigate to node_apple. Set is_end=False.
        node_apple: no children, is_end=False → prune it.
        node_appl: no children, is_end=False → prune it.
        node_app: no children... wait, is_end=True ("app" still exists) → STOP.

    Result: "app" remains; "apple" is gone; node_app still exists.

=== Longest Prefix Match ===

Walk the input string character by character, recording the last is_end node seen.
Stop when a character is not found in the current node's children.

Used in IP routing: a router stores route prefixes and finds the most specific
matching prefix for each incoming packet — O(k) per packet.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Generic, Hashable, Iterator, TypeVar

K = TypeVar("K", bound=Hashable)
V = TypeVar("V")


# ─── Exceptions ──────────────────────────────────────────────────────────────


class TrieError(Exception):
    """Base exception for all Trie errors."""


class KeyNotFoundError(TrieError):
    """Raised when a key is not found and a value was expected."""


# ─── Internal node ───────────────────────────────────────────────────────────


@dataclass
class _TrieNode(Generic[V]):
    """
    Internal node of the trie.

    Each node represents one character on a path from the root to a key. The
    node itself does not store which character it represents — that information
    is held by the PARENT as the key in its `children` dict.

    Attributes:
        children: Maps a single character to the child node for that character.
        is_end:   True if a complete key ends at this node.
        value:    The value stored for the key ending here (None if is_end=False).
    """

    children: dict[str, "_TrieNode[V]"] = field(default_factory=dict)
    is_end: bool = False
    value: V | None = None


# ─── Main class ───────────────────────────────────────────────────────────────


class Trie(Generic[V]):
    """
    Prefix tree (trie) mapping string keys to values of type V.

    Unlike a dict, a Trie supports prefix operations:
      - Find all keys starting with a prefix (autocomplete)
      - Check if any key starts with a prefix
      - Find the longest stored key that is a prefix of a given string (IP routing)

    Keys are arbitrary strings (Unicode supported). Values are arbitrary.
    If you only need word membership (no values), use Trie[bool] with value=True.

    Examples:
        >>> t = Trie()
        >>> t.insert("apple", 1)
        >>> t.insert("app", 2)
        >>> t.search("apple")
        1
        >>> t.search("ap")    # not a complete key
        >>> t.starts_with("ap")
        True
        >>> t.words_with_prefix("app")
        [('app', 2), ('apple', 1)]
        >>> t.delete("app")
        True
        >>> len(t)
        1
    """

    def __init__(self) -> None:
        """Create an empty trie."""
        self._root: _TrieNode[V] = _TrieNode()
        self._size: int = 0

    # ── Core operations ──────────────────────────────────────────────────────

    def insert(self, key: str, value: V = True) -> None:  # type: ignore[assignment]
        """
        Store key → value. If key already exists, update its value.

        Creates one node per new character in the key. If the entire key path
        already exists, only updates is_end and value at the terminal node.

        Args:
            key:   The string key to insert (any Unicode string, including "").
            value: Value to associate with the key. Defaults to True for
                   simple word-set use cases.

        Time: O(len(key))

        Example:
            t.insert("apt")   # root→a→p→t(*), 1 new node created
            t.insert("app")   # root→a→p already exists; only add 'p' child
        """
        node = self._root
        for char in key:
            if char not in node.children:
                node.children[char] = _TrieNode()
            node = node.children[char]
        if not node.is_end:
            self._size += 1
        node.is_end = True
        node.value = value

    def search(self, key: str) -> V | None:
        """
        Exact match lookup. Returns value if key exists, None otherwise.

        'app' returns None if only 'apple' is stored — the path exists but
        the 'app' node is not marked as a word end.

        Args:
            key: The string key to search for.

        Returns:
            The stored value if key exists, otherwise None.

        Time: O(len(key))

        Example:
            t.insert("apple", 42)
            t.search("apple")  → 42
            t.search("app")    → None (no is_end at the 'app' node)
            t.search("apples") → None (path doesn't exist)
        """
        node = self._find_node(key)
        if node is None or not node.is_end:
            return None
        return node.value

    def delete(self, key: str) -> bool:
        """
        Remove key from the trie. Returns False if key not found.

        After removing the key, prunes any nodes that are now useless:
        nodes with no children and no is_end flag.

        Args:
            key: The key to remove.

        Returns:
            True if the key was found and deleted, False if it wasn't present.

        Time: O(len(key))

        Example:
            After inserting "app" and "apple":
            delete("app")  → True; node at 'p'(third) still exists (has child 'l')
            delete("xyz")  → False (not found)
        """
        if self.search(key) is None:
            return False
        self._delete_helper(self._root, key, 0)
        self._size -= 1
        return True

    def starts_with(self, prefix: str) -> bool:
        """
        Return True if any stored key starts with the given prefix.

        The empty string "" is a prefix of every string, so starts_with("")
        returns True whenever the trie is non-empty.

        Args:
            prefix: The prefix to check.

        Returns:
            True if at least one stored key starts with prefix.

        Time: O(len(prefix))

        Example:
            After inserting "apple":
            starts_with("app")  → True
            starts_with("apz")  → False
            starts_with("")     → True  (empty prefix matches everything)
        """
        if not prefix:
            return self._size > 0
        return self._find_node(prefix) is not None

    def words_with_prefix(self, prefix: str) -> list[tuple[str, V]]:
        """
        Return all (key, value) pairs where the key starts with prefix.

        Results are returned in lexicographic order because the DFS visits
        children in sorted order.

        Args:
            prefix: The prefix to search for. Empty string returns all words.

        Returns:
            List of (key, value) pairs, sorted lexicographically.

        Time: O(len(prefix) + total characters in results)

        Example:
            t.insert("app", 1); t.insert("apple", 2); t.insert("apt", 3)
            t.words_with_prefix("app") → [("app", 1), ("apple", 2)]
            t.words_with_prefix("xy")  → []
        """
        node = self._find_node(prefix)
        if node is None:
            return []
        results: list[tuple[str, V]] = []
        self._collect_all(node, prefix, results)
        return results

    def longest_prefix_match(self, string: str) -> tuple[str, V] | None:
        """
        Return (key, value) where key is the longest stored key that is a
        prefix of `string`. Returns None if no stored key is a prefix of string.

        Walk character by character through string, tracking the last node where
        is_end=True (the last complete key encountered along the path). Stop when
        a character is not found in the current node's children.

        Args:
            string: The input string to match against stored keys.

        Returns:
            (longest_matching_key, value) or None if no match.

        Time: O(len(string))

        Use cases: IP routing tables, URL dispatch, command parsing.

        Example:
            t.insert("a", 1); t.insert("ab", 2); t.insert("abc", 3)
            t.longest_prefix_match("abcdef") → ("abc", 3)
            t.longest_prefix_match("xyz")    → None
            t.longest_prefix_match("a")      → ("a", 1)
        """
        node = self._root
        last_match: tuple[str, V] | None = None
        current: list[str] = []

        for char in string:
            if char not in node.children:
                break
            node = node.children[char]
            current.append(char)
            if node.is_end:
                # node.value is not None when is_end is True
                last_match = ("".join(current), node.value)  # type: ignore[assignment]

        return last_match

    def all_words(self) -> list[tuple[str, V]]:
        """
        Return all (key, value) pairs in lexicographic order.

        Equivalent to words_with_prefix("") except it also handles the empty
        string key correctly.

        Returns:
            All stored (key, value) pairs sorted lexicographically.

        Time: O(n · k) where n = number of keys, k = average key length.
        """
        results: list[tuple[str, V]] = []
        self._collect_all(self._root, "", results)
        return results

    # ── Dict-like interface ───────────────────────────────────────────────────

    def __contains__(self, key: object) -> bool:
        """Return True if key exists in the trie (exact match required)."""
        if not isinstance(key, str):
            return False
        return self.search(key) is not None

    def __getitem__(self, key: str) -> V:
        """
        Return value for key.

        Raises:
            KeyNotFoundError: If key is not in the trie.
        """
        value = self.search(key)
        if value is None and not self._key_exists(key):
            raise KeyNotFoundError(f"Key not found: {key!r}")
        return value  # type: ignore[return-value]

    def __setitem__(self, key: str, value: V) -> None:
        """Insert or update key → value."""
        self.insert(key, value)

    def __delitem__(self, key: str) -> None:
        """
        Remove key.

        Raises:
            KeyNotFoundError: If key is not in the trie.
        """
        if not self.delete(key):
            raise KeyNotFoundError(f"Key not found: {key!r}")

    # ── Iteration ─────────────────────────────────────────────────────────────

    def __iter__(self) -> Iterator[str]:
        """Iterate all keys in lexicographic order."""
        for key, _ in self.all_words():
            yield key

    def items(self) -> Iterator[tuple[str, V]]:
        """Iterate (key, value) pairs in lexicographic order."""
        yield from self.all_words()

    # ── Metadata ──────────────────────────────────────────────────────────────

    def __len__(self) -> int:
        """Return number of unique keys stored. O(1)."""
        return self._size

    def __bool__(self) -> bool:
        """Return True if any key is stored."""
        return self._size > 0

    def __repr__(self) -> str:
        """Developer-friendly representation."""
        words = self.all_words()
        preview = words[:5]
        suffix = f", ...+{len(words) - 5}" if len(words) > 5 else ""
        return f"Trie({self._size} keys: {preview}{suffix})"

    def is_valid(self) -> bool:
        """
        Verify structural invariants. Intended for testing only.

        Checks:
        1. The word count tracked in _size equals the actual count of is_end
           nodes in the tree.
        2. No node with is_end=False that has no children (orphan nodes after
           deletions should have been cleaned up).

        Returns:
            True if all invariants hold.

        Time: O(n · k)
        """
        actual_count = self._count_endpoints(self._root)
        return actual_count == self._size

    # ── Private helpers ───────────────────────────────────────────────────────

    def _find_node(self, key: str) -> _TrieNode[V] | None:
        """
        Navigate to the node at the end of key's path.

        Returns None if any character in the path is missing.
        Does NOT check is_end — callers must do that themselves.

        Args:
            key: String key to navigate to.

        Returns:
            The terminal node, or None if the path doesn't exist.
        """
        node: _TrieNode[V] = self._root
        for char in key:
            if char not in node.children:
                return None
            node = node.children[char]
        return node

    def _collect_all(
        self,
        node: _TrieNode[V],
        current: str,
        results: list[tuple[str, V]],
    ) -> None:
        """
        DFS from node, collecting all (key, value) pairs in the subtree.

        Children are visited in sorted order, guaranteeing lexicographic output
        without an explicit sort step. This is O(k) per character traversed.

        Args:
            node:    Current node in the DFS.
            current: The string prefix built so far (key for this node's level).
            results: Output list to append (key, value) pairs to.
        """
        if node.is_end:
            # node.value is not None when is_end is True
            results.append((current, node.value))  # type: ignore[arg-type]
        for char in sorted(node.children):
            self._collect_all(node.children[char], current + char, results)

    def _delete_helper(
        self,
        node: _TrieNode[V],
        key: str,
        depth: int,
    ) -> bool:
        """
        Recursively delete key[depth:] from the subtree rooted at node.

        Returns True if this node should be removed from its parent's children
        dict (because it has no children and is no longer a word endpoint).

        Args:
            node:  Current node.
            key:   The full key being deleted.
            depth: Current character index.

        Returns:
            True if the caller should remove node from its parent.

        Strategy:
            - Base case (depth == len(key)): unmark is_end. If no children
              remain, tell parent to remove this node.
            - Recursive case: recurse into the child for key[depth]. If child
              says to remove it, do so. Then check if this node is now
              removable (no children, not an endpoint).
        """
        if depth == len(key):
            # We've reached the terminal node — unmark it as a word end.
            node.is_end = False
            node.value = None
            # If no children, this node serves no purpose — remove it.
            return not node.children

        char = key[depth]
        child = node.children.get(char)
        if child is None:
            return False  # shouldn't happen if we checked exists first

        should_remove_child = self._delete_helper(child, key, depth + 1)
        if should_remove_child:
            del node.children[char]

        # This node is removable if it has no children and isn't a word end.
        return not node.children and not node.is_end

    def _count_endpoints(self, node: _TrieNode[V]) -> int:
        """
        Count all is_end=True nodes in the subtree.

        Used only by is_valid() to verify the _size counter.

        Args:
            node: Root of the subtree to count.

        Returns:
            Number of is_end=True nodes in the subtree.
        """
        count = 1 if node.is_end else 0
        for child in node.children.values():
            count += self._count_endpoints(child)
        return count

    def _key_exists(self, key: str) -> bool:
        """
        Return True if key exists (including when its value is None).

        Used by __getitem__ to distinguish "key with value None" from "key absent".
        """
        node = self._find_node(key)
        return node is not None and node.is_end


# ─── TrieCursor ───────────────────────────────────────────────────────────────


@dataclass
class _CursorNode(Generic[K, V]):
    """
    Internal node of a TrieCursor's trie.

    Generic over key element type K (e.g., int for bytes, str for characters)
    and value type V. The key element is stored in the PARENT's `children` dict,
    not in the node itself — the same design as _TrieNode.

    Attributes:
        children: Maps one key element to the child node for that element.
        value:    The value stored at this node (None if no value).
    """

    children: dict[Any, "_CursorNode[K, V]"] = field(default_factory=dict)
    value: V | None = None


class TrieCursor(Generic[K, V]):
    """
    A cursor for step-by-step trie traversal.

    Unlike ``Trie`` (which inserts and searches complete keys at once),
    ``TrieCursor`` maintains a *current position* in a trie and advances one
    key element at a time. This is the core primitive for streaming algorithms
    that incrementally match sequences against a growing dictionary:

    - **LZ78** (CMP01): encode input byte-by-byte, emitting a token whenever
      the current byte has no child edge.
    - **LZW** (CMP03): same pattern with a pre-seeded 256-entry alphabet.
    - **Aho-Corasick**: walk an input string against a trie of patterns.

    The cursor is self-contained: it owns its own trie root internally and
    builds the trie as you call :meth:`insert`.

    === Cursor state machine ===

        (at root)
            │
            ├─ step(e) → True   ──→  (at child for e)
            ├─ step(e) → False  ──→  (still at current node; child missing)
            │
            ├─ insert(e, v)  ──→  add child edge e with value v; move there
            ├─ reset()       ──→  (at root)
            └─ value         ──→  value at current position (None at root)

    === Example: LZ78 encoding ===

        cursor: TrieCursor[int, int] = TrieCursor()
        next_id = 1
        for byte in data:
            if not cursor.step(byte):
                emit Token(cursor.value or 0, byte)
                cursor.insert(byte, next_id)  # add dict entry
                next_id += 1
                cursor.reset()
        if not cursor.at_root:
            emit Token(cursor.value or 0, 0)   # flush sentinel

    Type Parameters:
        K: The type of each key element (``int`` for bytes, ``str`` for chars).
        V: The type of values stored at each node.

    Examples:
        >>> cursor: TrieCursor[int, str] = TrieCursor()
        >>> cursor.step(65)        # 'A' — no child yet
        False
        >>> cursor.insert(65, "A-entry")   # add root → 65 → node("A-entry")
        >>> cursor.reset()
        >>> cursor.step(65)        # now follows the edge
        True
        >>> cursor.value
        'A-entry'
    """

    def __init__(self) -> None:
        """Create a TrieCursor with an empty trie. Cursor starts at root."""
        self._root: _CursorNode[K, V] = _CursorNode()
        self._current: _CursorNode[K, V] = self._root

    # ── Navigation ────────────────────────────────────────────────────────────

    def step(self, element: K) -> bool:
        """
        Try to follow the child edge for ``element`` from the current position.

        If the child exists, the cursor advances to that child and returns
        ``True``. If not, the cursor stays at the current position and returns
        ``False``.

        Args:
            element: One key element (e.g., a byte value or a character).

        Returns:
            ``True`` if a child edge for ``element`` exists and was followed.
            ``False`` if no such edge exists (cursor position unchanged).

        Time: O(1) average (dict lookup).

        Example:
            >>> cursor: TrieCursor[int, int] = TrieCursor()
            >>> cursor.step(65)
            False
            >>> cursor.insert(65, 1)
            >>> cursor.reset()
            >>> cursor.step(65)
            True
        """
        child = self._current.children.get(element)
        if child is None:
            return False
        self._current = child
        return True

    def insert(self, element: K, value: V) -> None:
        """
        Add a child edge for ``element`` at the current position.

        Creates a new child node with ``value`` and stores it under ``element``
        in the current node's children. The cursor does NOT advance after
        inserting — call :meth:`reset` to go back to root or :meth:`step` to
        follow the new edge.

        If a child for ``element`` already exists, its value is updated.

        Args:
            element: The key element for the new edge.
            value:   The value to store at the new node.

        Time: O(1) average.

        Example:
            >>> cursor: TrieCursor[int, int] = TrieCursor()
            >>> cursor.insert(65, 1)   # root → 65 → node(value=1)
            >>> cursor.step(65)
            True
            >>> cursor.value
            1
        """
        existing = self._current.children.get(element)
        if existing is None:
            self._current.children[element] = _CursorNode(value=value)
        else:
            existing.value = value

    def reset(self) -> None:
        """
        Reset the cursor to the root of the trie.

        After reset, :attr:`at_root` is ``True`` and :attr:`value` is
        whatever value was stored at the root (typically ``None``).

        Time: O(1).
        """
        self._current = self._root

    # ── Inspection ────────────────────────────────────────────────────────────

    @property
    def value(self) -> V | None:
        """
        The value stored at the current node.

        Returns ``None`` if no value was stored at this position (e.g., at the
        root of a freshly created cursor, or at an internal node that has
        children but no value itself).

        Time: O(1).
        """
        return self._current.value

    @property
    def at_root(self) -> bool:
        """
        Return ``True`` if the cursor is at the root node.

        Useful as an end-of-stream check in LZ78/LZW to detect whether a
        partial match needs to be flushed.

        Time: O(1).

        Example:
            >>> cursor: TrieCursor[int, int] = TrieCursor()
            >>> cursor.at_root
            True
            >>> cursor.insert(65, 1)
            >>> cursor.step(65)
            True
            >>> cursor.at_root
            False
        """
        return self._current is self._root

    # ── Iteration ─────────────────────────────────────────────────────────────

    def __iter__(self) -> Iterator[tuple[list[K], V]]:
        """
        Iterate over all (path, value) pairs stored in the trie.

        Yields tuples of ``([key_elements, ...], value)`` for every node that
        has a value (i.e., was created by :meth:`insert`). Traversal order is
        DFS, following children in insertion order.

        Does NOT include the root node (which has no meaningful path).

        Time: O(n) where n is the number of nodes in the trie.

        Example:
            >>> cursor: TrieCursor[int, str] = TrieCursor()
            >>> cursor.insert(65, "A")           # root → 65 → "A"
            >>> cursor.reset()
            >>> cursor.step(65)
            True
            >>> cursor.insert(66, "AB")          # root → 65 → 66 → "AB"
            >>> cursor.reset()
            >>> sorted(cursor, key=lambda p: p[1])
            [([65], 'A'), ([65, 66], 'AB')]
        """
        yield from self._iter_node(self._root, [])

    def _iter_node(
        self,
        node: _CursorNode[K, V],
        path: list[K],
    ) -> Iterator[tuple[list[K], V]]:
        if node.value is not None:
            yield (list(path), node.value)
        for element, child in node.children.items():
            path.append(element)
            yield from self._iter_node(child, path)
            path.pop()

    def __len__(self) -> int:
        """Return the number of nodes that have a value stored. O(n)."""
        return sum(1 for _ in self)

    def __bool__(self) -> bool:
        """Return True if any node has a value stored."""
        return bool(self._root.children)
