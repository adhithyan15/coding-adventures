"""
radix_tree.py — Radix Tree (Compressed Trie / Patricia Trie)
=============================================================

A **radix tree** is a trie (DT13) where chains of single-child nodes are
collapsed into single edges labeled with the full substring they represented.

=== The Core Idea ===

Consider the trie for ["search", "searcher", "searching"]:

    Trie (DT13) — 14 nodes for 3 words, most nodes have exactly 1 child:
      root
      └── s
          └── e
              └── a
                  └── r
                      └── c
                          └── h (is_end)
                              ├── e
                              │   └── r (is_end)
                              └── i
                                  └── n
                                      └── g (is_end)

    Radix tree (DT14) — 4 nodes, edges hold full substrings:
      root
      └── "search" (is_end)
          ├── "er"  (is_end)
          └── "ing" (is_end)

The trie uses one node per character. The radix tree collapses every
"chain" — a sequence of nodes where each node has exactly one child — into
a single edge carrying the full label of that chain.

This is crucial for:
  - Memory efficiency: URL routers, IP routing tables, file-path indexes
  - Redis stores all its keys in a radix tree (rax.c)
  - HTTP routers (gorilla/mux, httprouter, actix-web) use radix trees for
    path dispatching — O(path length) regardless of how many routes exist

=== Node Design ===

Each node stores:
  is_end:  True if a complete key ends at this node
  value:   The associated value (only meaningful when is_end=True)
  children: dict mapping the FIRST CHARACTER of each edge label to
            a (full_label, child_node) pair.

    Why index by first character? No two edges from the same node can share
    a first character — if they did, they'd share a common prefix and should
    be merged. So the first character is a unique key, enabling O(1) lookup.

    children = {
        'a': ("apple",    child_1),   # first char 'a' → full label "apple"
        'b': ("banana",   child_2),   # first char 'b' → full label "banana"
        'c': ("cherry",   child_3),   # first char 'c' → full label "cherry"
    }

=== The Four Insertion Cases ===

When inserting key K and there is an existing edge with label L from the
current node, we compute P = longest_common_prefix(K, L) and branch:

    Case 1: P = "" — no common prefix
      K and L start with different characters. Add K as a new edge.

    Case 2: P = L — L is a prefix of K (K extends L)
      Descend through the L edge and continue inserting K[len(L):].

    Case 3: P = K — K is a prefix of L (K ends in the middle of L)
      SPLIT: create a new intermediate node. The old edge L splits into
      two edges: P (the new key ends here) and L[len(P):] (old subtree).

    Case 4: P is a strict prefix of both K and L (partial match)
      SPLIT: create a new intermediate node. Both K[len(P):] and L[len(P):]
      become children of the new node.

=== Delete and Merge ===

After deleting a key (clearing is_end), check if the node is now an inner
node with exactly one child. If so, MERGE: combine the parent edge and the
single child's edge into one longer edge.

    Before: root → "app" (is_end=True) → "le" (is_end=True)
    Delete "app":
      "app" node: is_end=False, 1 child ("le") → MERGE
    After: root → "apple" (is_end=True)

This merge is the reverse of a Case 3 split.

=== Relation to DT13 Trie ===

The radix tree is a compressed trie. Every radix tree represents exactly the
same key/value mapping as the trie formed by expanding each edge label
character by character. The compression does not change semantics — only
the internal representation and memory usage.

    Trie insert:  O(k)  — create one node per character
    Radix insert: O(k)  — but may split one existing node (constant overhead)
    Trie search:  O(k)  — follow one child per character
    Radix search: O(k)  — compare edge labels (still O(k) total comparisons)

The radix tree therefore does NOT improve asymptotic complexity — it reduces
constant factors (fewer nodes = fewer allocations = better cache behaviour).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Generic, Iterator, TypeVar

V = TypeVar("V")


# ─── Helper ───────────────────────────────────────────────────────────────────


def _common_prefix_len(a: str, b: str) -> int:
    """
    Return the length of the longest common prefix of strings a and b.

    Examples:
        _common_prefix_len("apple", "application") → 4  ("appl")
        _common_prefix_len("apple", "banana")      → 0
        _common_prefix_len("abc", "abc")            → 3
        _common_prefix_len("", "abc")               → 0

    Time: O(min(len(a), len(b)))
    """
    i = 0
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    return i


# ─── Node ─────────────────────────────────────────────────────────────────────


@dataclass
class RadixNode(Generic[V]):
    """
    A single node in a radix tree.

    Each edge from this node is stored as (full_edge_label, child_node), indexed
    in the `children` dict by the FIRST CHARACTER of the edge label. Because no
    two edges from the same node can share a first character (they would need to
    be merged), this indexing gives O(1) child lookup.

    Attributes:
        is_end:   True when a complete key ends at this node.
        value:    The value associated with the key ending here (None otherwise).
        children: Maps first_char → (full_label, child_node).

    Example — node reached by "appl":
        RadixNode(
            is_end=False,
            value=None,
            children={
                'e': ("e",       leaf_for_apple),      # "apple"
                'i': ("ication", leaf_for_application) # "application"
            }
        )
    """

    children: dict[str, tuple[str, "RadixNode[V]"]] = field(default_factory=dict)
    is_end: bool = False
    value: V | None = None


def _make_leaf(value: V) -> RadixNode[V]:
    """Create a terminal leaf node storing the given value."""
    return RadixNode(is_end=True, value=value)


def _make_inner() -> RadixNode[V]:
    """Create an inner node (not an endpoint)."""
    return RadixNode(is_end=False, value=None)


# ─── Main class ───────────────────────────────────────────────────────────────


class RadixTree(Generic[V]):
    """
    Radix tree (compressed trie / Patricia trie) mapping string keys to values.

    Like a trie, it supports all prefix operations in O(key length) time.
    Unlike a trie, it uses O(number of keys) nodes instead of O(total characters),
    which is critical when keys share long prefixes or have long unique tails.

    ==========================================================================
    Quick-reference ASCII layout
    ==========================================================================

    After inserting ["app"→1, "apple"→2, "application"→3]:

        root (is_end=False)
        └── "app" (is_end=True, value=1)
            ├── "le"       (is_end=True, value=2)   ← "apple"
            └── "lication" (is_end=True, value=3)   ← "application"

    Just 3 nodes instead of a 15-node trie.
    ==========================================================================

    Examples:
        >>> t: RadixTree[int] = RadixTree()
        >>> t.insert("apple", 1)
        >>> t.insert("app", 2)
        >>> t.search("apple")
        1
        >>> t.search("ap")    # not a complete key
        >>> t.starts_with("ap")
        True
        >>> t.words_with_prefix("app")
        ['app', 'apple']
        >>> t.delete("app")
        True
        >>> len(t)
        1
    """

    def __init__(self) -> None:
        """Create an empty radix tree."""
        self._root: RadixNode[V] = _make_inner()
        self._size: int = 0

    # ── Core operations ──────────────────────────────────────────────────────

    def insert(self, key: str, value: V) -> None:
        """
        Store key → value. If key already exists, update its value.

        The algorithm descends the tree matching the key against edge labels.
        When an edge partially matches (Cases 3 & 4), the edge is SPLIT to
        create an intermediate node. When the key extends an edge exactly
        (Case 2), we simply recurse deeper.

        Args:
            key:   The string key to insert (any Unicode string, including "").
            value: Value to associate with the key.

        Time: O(len(key))

        Example — inserting "app" when "apple" exists (Case 3 split):

            Before:  root → "apple" (is_end=True)
            Insert "app" → 99:

              P = common_prefix("app", "apple") = "app" = key
              Case 3: split "apple" at 3
              Split node (is_end=True, value=99)
              └── "le" → old leaf (is_end=True, unchanged)

            After:   root → "app" (is_end=True, value=99)
                            └── "le" (is_end=True, old value)
        """
        self._root, added = self._insert(self._root, key, value)
        if added:
            self._size += 1

    def _insert(
        self, node: RadixNode[V], key: str, value: V
    ) -> tuple[RadixNode[V], bool]:
        """
        Recursively insert key into the subtree rooted at node.

        Returns (new_root, was_new_key_added).
        """
        if key == "":
            # The key ends at this node.
            added = not node.is_end
            node.is_end = True
            node.value = value
            return node, added

        first = key[0]

        if first not in node.children:
            # Case 1: No matching edge at all. Add a new leaf edge.
            #
            #   Before: node (no 'f' child)
            #   Insert "foo":
            #   After:  node → "foo" (is_end=True)
            node.children[first] = (key, _make_leaf(value))
            return node, True

        label, child = node.children[first]
        p_len = _common_prefix_len(key, label)

        if p_len == len(label):
            # Case 2: The full edge label is consumed by the key.
            # Example: edge is "app", key is "application".
            # Descend into child with remaining key "lication".
            #
            #   Before: node → "app" → child ...
            #   Insert "application":
            #   Navigate through "app", then insert "lication" into child.
            new_child, added = self._insert(child, key[p_len:], value)
            node.children[first] = (label, new_child)
            return node, added

        # Cases 3 & 4: partial match — we must SPLIT the edge.
        #
        # common = the shared prefix of key and label.
        # key_rest   = key[p_len:]   — remaining part of the new key (may be "")
        # label_rest = label[p_len:] — remaining part of the old label
        #
        # We create a new intermediate node at the split point.
        # The new node's children are:
        #   label_rest[0] → (label_rest, old_child)
        #   key_rest[0]   → (key_rest, new_leaf)   [only if key_rest != ""]
        #
        # Case 3: key_rest == "" → the new key ends exactly at the split point.
        #   Example: insert "app" when "apple" exists.
        #   common="app", label_rest="le", key_rest=""
        #   split_node is_end=True (the new key ends here)
        #
        # Case 4: key_rest != "" → both key and label extend past the split point.
        #   Example: insert "apple" when "application" exists.
        #   common="appl", label_rest="ication", key_rest="e"
        #   split_node is_end=False (no key ends at "appl")
        #
        # Diagram (Case 4):
        #   Before: node → "application" (is_end)
        #   After:  node → "appl" (inner)
        #                   ├── "ication" (is_end)  ← old child
        #                   └── "e"      (is_end)  ← new leaf

        common = label[:p_len]
        label_rest = label[p_len:]
        key_rest = key[p_len:]

        # Build the split node.
        if key_rest == "":
            # Case 3: new key ends exactly at the split point.
            split_node: RadixNode[V] = RadixNode(is_end=True, value=value)
            split_node.children[label_rest[0]] = (label_rest, child)
            node.children[first] = (common, split_node)
            return node, True
        else:
            # Case 4: both diverge — split node is NOT an endpoint.
            split_node = _make_inner()
            split_node.children[label_rest[0]] = (label_rest, child)
            split_node.children[key_rest[0]] = (key_rest, _make_leaf(value))
            node.children[first] = (common, split_node)
            return node, True

    def search(self, key: str) -> V | None:
        """
        Exact-match lookup. Returns value if key exists, None otherwise.

        Searching for "app" in a tree that only has "apple" returns None —
        the path may partially match an edge label, but the key must align
        exactly with a node that has is_end=True.

        Args:
            key: The string key to look up.

        Returns:
            The stored value if key is found, otherwise None.

        Time: O(len(key))
        """
        node = self._root
        remaining = key

        while remaining:
            first = remaining[0]
            if first not in node.children:
                return None
            label, child = node.children[first]
            p_len = _common_prefix_len(remaining, label)
            if p_len < len(label):
                # Key runs out before the edge label ends — no match.
                return None
            remaining = remaining[p_len:]
            node = child

        return node.value if node.is_end else None

    def delete(self, key: str) -> bool:
        """
        Remove key from the tree. Returns True if deleted, False if not found.

        After clearing is_end, walks back up and MERGES any node that is now
        a non-endpoint inner node with exactly one child. This is the inverse
        of the Case 3 split:

            Before: root → "apple" (is_end=True)
            Suppose "app" also exists:
            root → "app" (is_end=True) → "le" (is_end=True)

            Delete "app":
              Clear is_end at "app" node.
              "app" node: is_end=False, 1 child → MERGE with "le" child.

            After: root → "apple" (is_end=True)

        Args:
            key: The key to remove.

        Returns:
            True if the key existed and was removed, False otherwise.

        Time: O(len(key))
        """
        deleted, _ = self._delete(self._root, key)
        if deleted:
            self._size -= 1
        return deleted

    def _delete(
        self, node: RadixNode[V], key: str
    ) -> tuple[bool, bool]:
        """
        Recursively delete key from the subtree rooted at node.

        Returns (was_deleted, should_merge_this_node_with_parent).

        The "should merge" signal is True when this node has become a
        non-endpoint with exactly one child after deletion — in that case the
        parent should absorb this node's single edge into its own edge to
        this node.

        We implement merging at the parent level (in the recursive call's
        return path) so we always have access to both the parent's edge label
        and the child's single remaining edge label.
        """
        if key == "":
            # Reached the terminal node.
            if not node.is_end:
                return False, False
            node.is_end = False
            node.value = None
            # Signal merge if this node now has exactly one child and is not end.
            mergeable = len(node.children) == 1
            return True, mergeable

        first = key[0]
        if first not in node.children:
            return False, False

        label, child = node.children[first]
        p_len = _common_prefix_len(key, label)

        if p_len < len(label):
            # Key doesn't fully traverse this edge — key not in tree.
            return False, False

        deleted, child_mergeable = self._delete(child, key[p_len:])
        if not deleted:
            return False, False

        if child_mergeable:
            # The child node has exactly one child and is not an endpoint.
            # Merge: replace (label → child → child's_only_edge → grandchild)
            # with   (label + child_edge_label → grandchild).
            assert len(child.children) == 1
            (grandchild_label, grandchild) = next(iter(child.children.values()))
            merged_label = label + grandchild_label
            node.children[first] = (merged_label, grandchild)
        elif not child.is_end and len(child.children) == 0:
            # Child became a dead leaf (no end, no children) — prune it.
            del node.children[first]

        # This node becomes mergeable if it's now a non-endpoint with 1 child.
        mergeable = not node.is_end and len(node.children) == 1
        return True, mergeable

    def starts_with(self, prefix: str) -> bool:
        """
        Return True if any stored key starts with prefix.

        The empty prefix matches everything (returns True when tree is non-empty).

        Args:
            prefix: The prefix string to check.

        Returns:
            True if at least one stored key begins with prefix.

        Time: O(len(prefix))

        Example:
            After inserting "application":
            starts_with("app")   → True
            starts_with("apple") → False (no key starts with "apple" here)
            starts_with("")      → True (empty prefix matches everything)
        """
        if not prefix:
            return self._size > 0

        node = self._root
        remaining = prefix

        while remaining:
            first = remaining[0]
            if first not in node.children:
                return False
            label, child = node.children[first]
            p_len = _common_prefix_len(remaining, label)
            if p_len == len(remaining):
                # We consumed the prefix — it either aligns with an edge end
                # or we're in the middle of an edge. Either way, a key exists below.
                return True
            if p_len < len(label):
                # Prefix doesn't match this edge fully OR partially mismatches.
                return False
            remaining = remaining[p_len:]
            node = child

        # remaining == "" — we're at a node that is either an endpoint or has children.
        return node.is_end or bool(node.children)

    def words_with_prefix(self, prefix: str) -> list[str]:
        """
        Return all stored keys that start with prefix, in lexicographic order.

        Algorithm:
          1. Walk the tree consuming the prefix, tracking any partial edge match.
          2. From the node (and partial position within any edge), DFS to collect
             all keys in the subtree, prepending the accumulated path.

        Args:
            prefix: Prefix to filter by. Empty string returns all keys.

        Returns:
            Sorted list of keys matching the prefix.

        Time: O(len(prefix) + total characters in result keys)

        Example:
            After inserting ["search"→1, "searcher"→2, "searching"→3]:
            words_with_prefix("search")     → ["search", "searcher", "searching"]
            words_with_prefix("searcher")   → ["searcher"]
            words_with_prefix("searching")  → ["searching"]
            words_with_prefix("xyz")        → []
            words_with_prefix("")           → ["search", "searcher", "searching"]
        """
        node = self._root
        remaining = prefix
        accumulated = prefix  # will hold the full path to the subtree root

        while remaining:
            first = remaining[0]
            if first not in node.children:
                return []
            label, child = node.children[first]
            p_len = _common_prefix_len(remaining, label)

            if p_len == len(remaining):
                # Prefix ends in the middle of (or at the end of) this edge.
                # Keys below this child continue with label[p_len:] appended.
                if p_len == len(label):
                    # Prefix exactly consumed this edge — descend normally.
                    node = child
                    remaining = ""
                else:
                    # Prefix ends mid-edge: the subtree below `child` has all
                    # keys that start with `prefix`. The prefix extends into
                    # the edge label. We treat `child` as the collection root
                    # but remember the edge suffix (label[p_len:]) is already
                    # part of every key that passes through `child`.
                    edge_suffix = label[p_len:]
                    results: list[str] = []
                    self._collect(child, accumulated + edge_suffix, results)
                    return results
            elif p_len < len(label):
                # Mismatch — no keys match this prefix.
                return []
            else:
                # p_len == len(label) — fully consumed this edge, keep going.
                remaining = remaining[p_len:]
                node = child

        results = []
        self._collect(node, accumulated, results)
        return results

    def longest_prefix_match(self, key: str) -> str | None:
        """
        Return the longest stored key that is a prefix of `key`.

        Walks through `key` character by character following edges. At each
        node, if is_end=True, record the current position as a candidate match.
        Stop when no edge matches the next character of `key`.

        Use cases:
          - IP routing: store route prefixes, match incoming packet addresses
          - URL dispatch: store URL prefixes, match request paths
          - Command parsing: "help" matches prefix "h" if "h" is registered

        Args:
            key: The input string to match against stored keys.

        Returns:
            The longest stored key that is a prefix of `key`, or None.

        Time: O(len(key))

        Example:
            After inserting ["a", "ab", "abc"]:
            longest_prefix_match("abcdef") → "abc"
            longest_prefix_match("xyz")    → None
            longest_prefix_match("a")      → "a"
            longest_prefix_match("ab")     → "ab"
        """
        node = self._root
        remaining = key
        consumed = 0  # characters of `key` consumed so far
        best: str | None = None

        if node.is_end:
            best = ""

        while remaining:
            first = remaining[0]
            if first not in node.children:
                break
            label, child = node.children[first]
            p_len = _common_prefix_len(remaining, label)
            if p_len < len(label):
                # We can partially traverse this edge only if remaining runs
                # out before the edge does. Check if any prefix of the edge
                # matches — but since is_end can only be at a NODE (not mid-edge),
                # a partial edge match yields no new endpoint.
                break
            # We fully consumed this edge label.
            consumed += p_len
            remaining = remaining[p_len:]
            node = child
            if node.is_end:
                best = key[:consumed]

        return best

    # ── Dict-like interface ───────────────────────────────────────────────────

    def __len__(self) -> int:
        """Return the number of unique keys stored. O(1)."""
        return self._size

    def __contains__(self, key: object) -> bool:
        """Return True if key exists in the tree (exact match required)."""
        if not isinstance(key, str):
            return False
        return self.search(key) is not None

    def __iter__(self) -> Iterator[str]:
        """
        Yield all keys in lexicographic order.

        Achieved by visiting children in sorted order of their first character
        during DFS. Because no two edges from the same node share a first
        character, sorted(node.children) gives lexicographic edge order.
        """
        results: list[str] = []
        self._collect(self._root, "", results)
        yield from results

    def to_dict(self) -> dict[str, V]:
        """
        Return a plain dict with all (key, value) pairs.

        Useful for serialisation, debugging, and comparison with expected values.

        Returns:
            dict mapping every stored key to its value.

        Time: O(n · k) where n = number of keys, k = average key length.
        """
        result: dict[str, V] = {}
        self._collect_values(self._root, "", result)
        return result

    def __repr__(self) -> str:
        """Developer-friendly representation showing stored keys."""
        keys = list(self)
        preview = keys[:5]
        suffix = f", ...+{len(keys) - 5}" if len(keys) > 5 else ""
        return f"RadixTree({self._size} keys: {preview}{suffix})"

    # ── Private helpers ───────────────────────────────────────────────────────

    def _collect(
        self,
        node: RadixNode[V],
        current_key: str,
        results: list[str],
    ) -> None:
        """
        DFS from node, collecting all keys in lexicographic order.

        Children are visited in sorted order of their first character.
        Because all edge labels from a node start with distinct first characters,
        sorted(node.children) gives lexicographic order across branches.

        For each edge (label, child), the accumulated key grows by label. When
        we reach a node with is_end=True, we emit the accumulated key.

        Args:
            node:        Current node in the DFS.
            current_key: The key string accumulated to reach this node.
            results:     Output list; keys are appended in sorted order.
        """
        if node.is_end:
            results.append(current_key)
        for first_char in sorted(node.children):
            label, child = node.children[first_char]
            self._collect(child, current_key + label, results)

    def _collect_values(
        self,
        node: RadixNode[V],
        current_key: str,
        result: dict[str, V],
    ) -> None:
        """
        DFS from node, collecting (key, value) pairs into result dict.

        Args:
            node:        Current node.
            current_key: Accumulated key path to this node.
            result:      Output dict to populate.
        """
        if node.is_end:
            result[current_key] = node.value  # type: ignore[assignment]
        for first_char in sorted(node.children):
            label, child = node.children[first_char]
            self._collect_values(child, current_key + label, result)
