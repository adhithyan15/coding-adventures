# DT14 — Radix Tree (Compressed Trie)

## Overview

A **radix tree** (also called a Patricia trie, compact prefix tree, or compressed
trie) is a trie (DT13) where chains of single-child nodes are collapsed into a
single edge labeled with the full substring they represented.

Imagine the trie for ["apple", "application"]:

```
Trie (DT13):
  root → a → p → p → l → e (end)
                       └ → i → c → a → t → i → o → n (end)

Radix tree (DT14):
  root → "app" → "le" (end)
               → "lication" (end)
```

The trie uses 13 nodes. The radix tree uses 3 nodes and 3 edges. The structure is
identical — but compressed. Every chain `a → b → c` where `a` has only one child `b`
and `b` has only one child `c` collapses to a single edge `"abc"`.

This compression is crucial when:
- Keys share long common prefixes (URLs, file paths, IP addresses)
- Keys have long unique tails (UUIDs, hashes, full sentences)
- Memory is constrained (embedded systems, in-memory databases)

**Redis** uses a radix tree (`rax.c`) to store all keys. When you run `KEYS app*`
on a Redis server with 10 million keys, Redis walks its radix tree to the "app"
subtree and collects entries — not a scan of all 10 million keys.

## Layer Position

```
DT13: trie            ← direct parent (radix tree compresses tries)
DT14: radix-tree      ← [YOU ARE HERE]
  └── DT15: suffix-tree  (radix tree of all suffixes of a string)

DT11: b-tree          (different family: disk-oriented, numeric keys)
DT18: hash-map        (no prefix operations; O(1) exact lookup)
```

**Depends on:** DT13 (trie: prefix-sharing tree, is_end markers, DFS collection).
**Extends:** DT13 with edge compression.
**Used by:** DT15 (suffix tree), Redis key storage, HTTP routers (gorilla/mux,
httprouter, actix-web), IP routing tables, autocomplete engines.

## Concepts

### Why Tries Waste Memory

Consider storing the keys ["search", "searcher", "searching"]:

```
Trie:
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
```

That is 14 nodes for 3 words with total length 23 characters. Most nodes have
exactly one child — they exist only because a trie creates one node per character.

Radix tree collapses all single-child chains:

```
  root
  └── "search" (is_end)
      ├── "er" (is_end)
      └── "ing" (is_end)
```

4 nodes instead of 14. Each edge stores a full substring, not a single character.

### Node Types

A radix tree has two kinds of nodes:

```
Inner node (has children):
  ┌───────────────────────────────────────────────┐
  │  is_end: bool     (True if a key ends HERE)   │
  │  value:  Any      (set if is_end is True)      │
  │  children: dict[str, RadixNode]               │
  │    key:   the FIRST CHARACTER of each edge    │
  │    value: (edge_label, child_node)            │
  └───────────────────────────────────────────────┘

Each edge is stored as a (label, child) pair.
The label is the full substring on that edge (e.g., "search", "er", "ing").
Children are indexed by their first character for O(1) lookup.

Leaf node (no children):
  is_end: True
  value:  Any
  children: {} (empty)
```

```
Edge label indexing (why index by first char):

  Node with children:
    "apple"    → child_1
    "banana"   → child_2
    "cherry"   → child_3

  To look up key "application":
    Take first char of remaining key: 'a'
    Look up children['a'] → ("apple", child_1)
    Compare "application" vs edge label "apple":
      "application"[0:5] == "apple"? YES → descend into child_1
      with remaining key "lication"
```

### The Four Insertion Cases

Insertion into a radix tree is more complex than a trie because edges hold
multiple characters. When inserting key K into a node that already has an
edge with label L, four cases arise:

```
Let P = common prefix of K and L (the longest prefix they share).

Case 1: P = "" (no common prefix)
  K and L share no characters at all.
  → Add K as a new edge from the current node.

  Before: node → "apple" → ...
  Insert "banana":
  After:  node → "apple" → ...
               → "banana" (end)

──────────────────────────────────────────────────────────────────────

Case 2: P = L (L is a prefix of K — K starts with L)
  K extends L further.
  → Descend into the child at edge L, continue inserting with K[len(L):].

  Before: node → "app" → "le" (end)
  Insert "application":
  After: navigate through "app", then continue from "app"'s child with "lication"
  Result: node → "app" → "le" (end)
                       → "lication" (end)

──────────────────────────────────────────────────────────────────────

Case 3: P = K (K is a prefix of L — L starts with K)
  K ends IN THE MIDDLE of an existing edge L.
  → SPLIT the edge: create a new inner node at the split point.
     Old edge becomes: K → new_inner → L[len(K):] → (old child)
     New key ends at: new_inner (mark it is_end=True)

  Before: node → "apple" → child_A
  Insert "app":
    P = "app" = K = "app".
    "apple"[0:3] = "app" = K. K is a prefix of L.
    Split "apple" into "app" + "le":

  After: node → "app" (is_end=True) → "le" → child_A
                 ↑
                 new inner node for "app"

  Verification:
    search("app")   → follows "app" edge, finds is_end=True ✓
    search("apple") → follows "app" edge, then "le" edge, finds end ✓

──────────────────────────────────────────────────────────────────────

Case 4: P is a strict prefix of BOTH K and L (partial match)
  K and L diverge at position len(P).
  → SPLIT: create a new inner node at the divergence point.
     Old edge is replaced by: P → new_inner → L[len(P):] → (old child)
     New key becomes:                        → K[len(P):] → (new end node)

  Before: node → "application" (end)
  Insert "apple":
    K = "apple", L = "application"
    Common prefix P = "appl" (both start with "appl", diverge at 'e' vs 'i')

  After: node → "appl" → "ication" (end)   ← old edge split
                       → "e" (end)          ← new key appended

  Complete picture:
    node → "appl" (inner, is_end=False)
               ├── "e"       (end — "apple")
               └── "ication" (end — "application")

  Verification:
    search("apple")       → "appl" → "e" → is_end ✓
    search("application") → "appl" → "ication" → is_end ✓
    search("appl")        → "appl" → not is_end → None ✓
    search("app")         → reaches common prefix "appl", doesn't match → None ✓
```

### Case 3 and 4 ASCII Diagrams in Detail

Case 3 (inserting a prefix of an existing edge) is subtle. Let's trace it step
by step with the actual node structure:

```
State: Radix tree contains only "apple" → value "fruit"

Internal representation:
  root (is_end=False)
  └── children['a'] = ("apple", leaf_node)
      leaf_node = {is_end=True, value="fruit", children={}}

Insert "app" → value "prefix"

Step 1: At root, look up children['a'] = ("apple", leaf_node)
Step 2: Compute common prefix of "app" and "apple":
         "app"   ← key being inserted
         "apple" ← existing edge label
         Common prefix: "app" (3 chars)
Step 3: Since len("app") == len("app") and "app" is a prefix of "apple":
         This is Case 3. Split "apple" at position 3.

Create split_node = {is_end=True, value="prefix", children={}}
  (is_end=True because "app" ends here)

Attach old remaining "le" → leaf_node as child of split_node:
  split_node.children['l'] = ("le", leaf_node)

Replace root's 'a' child:
  root.children['a'] = ("app", split_node)

Final state:
  root (is_end=False)
  └── children['a'] = ("app", split_node)
      split_node (is_end=True, value="prefix")
      └── children['l'] = ("le", leaf_node)
          leaf_node (is_end=True, value="fruit")
```

Case 4 (partial match, both keys diverge):

```
State: Radix tree contains "application" → value "program"

Insert "apple" → value "fruit"

Step 1: At root, look up children['a'] = ("application", end_node)
Step 2: Common prefix of "apple" and "application":
         "apple"        ← new key
         "application"  ← existing edge
         a p p l e        compare char by char
         a p p l i        ← diverge at position 4 ('e' vs 'i')
         Common prefix: "appl"
Step 3: len("appl")=4 < len("apple")=5 AND len("appl")=4 < len("application")=11
         This is Case 4. Split at position 4.

Create split_node = {is_end=False, children={}}
  (is_end=False because no key ends at "appl")

Remaining of old key: "application"[4:] = "ication"
  split_node.children['i'] = ("ication", end_node)
  end_node.value = "program" (unchanged)

Remaining of new key: "apple"[4:] = "e"
  new_leaf = {is_end=True, value="fruit", children={}}
  split_node.children['e'] = ("e", new_leaf)

Replace root's 'a' child:
  root.children['a'] = ("appl", split_node)

Final state:
  root
  └── "appl" → split_node (is_end=False)
               ├── "ication" → end_node (is_end=True, value="program")
               └── "e" → new_leaf (is_end=True, value="fruit")
```

### Deletion and Merging

When deleting a key from a radix tree, nodes may become mergeable: an inner node
that is no longer an endpoint and has exactly one child can be merged with its
single child into a single edge.

```
State: tree contains ["app", "apple"]
  root → "app" (is_end=True) → "le" (is_end=True)

Delete "app":
  Unmark is_end at "app" node.
  "app" node now has: is_end=False, exactly 1 child ("le").
  → MERGE: replace root's "app" edge + child's "le" edge
           with a single "apple" edge.

After merge:
  root → "apple" (is_end=True)

This merge is the inverse of a Case 3 or Case 4 split.
```

### Redis rax — Production Radix Tree

Redis stores every key you set (SET mykey value) in a radix tree. When you run:

```
KEYS user:*
```

Redis navigates to the "user:" node in its radix tree (O(p) time where p=6) and
collects all entries in that subtree. For a server with 5 million keys where
1,000 start with "user:", this is a radix tree DFS of 1,000 nodes — not a scan
of 5 million entries.

Redis's `rax.c` uses one more optimization: it stores short edge labels directly
in the node struct (no heap allocation for labels ≤ 44 bytes). This avoids a
pointer dereference for every edge traversal.

```
Redis rax node layout (simplified):
  struct raxNode {
      uint32_t iskey:1;      /* does a key end here? */
      uint32_t isnull:1;     /* is the value NULL? */
      uint32_t iscompr:1;    /* is this a compressed (single-child) node? */
      uint32_t size:29;      /* number of children OR length of compressed label */
      unsigned char data[];  /* edge labels + child pointers + optional value */
  };

Compressed node (iscompr=1): stores a multi-char edge label inline.
Non-compressed node (iscompr=0): stores one char per child (classic trie node).
```

The "compressed" vs "non-compressed" distinction is exactly the radix tree
optimization: if a node has one child, store the full label (compressed);
if it has multiple children, store one byte per child (uncompressed trie node).

### Comparison: Trie vs Radix Tree

```
Property                    │ Trie (DT13)         │ Radix Tree (DT14)
────────────────────────────┼─────────────────────┼────────────────────────
Node count                  │ O(total chars)      │ O(number of keys)
Memory (10K 10-char keys)   │ ~100K nodes         │ ~10K nodes (up to 10x less)
Insert (new unique suffix)  │ O(k)                │ O(k) with possible split
Insert (shared prefix)      │ O(k)                │ O(k) simpler (just descend)
Search                      │ O(k)                │ O(k) same asymptotic
Prefix query                │ O(p + results)      │ O(p + results) same
Implementation complexity   │ simple              │ more complex (4 split cases)
Best for                    │ small alphabets,    │ long keys, long shared prefixes,
                            │ educational use     │ production systems
```

## Representation

### Node

```python
@dataclass
class RadixNode:
    """
    A radix tree node.
    Each child is stored as (edge_label, child_node).
    children is indexed by the FIRST CHARACTER of the edge label for O(1) lookup.
    """
    children: dict[str, tuple[str, "RadixNode"]]
    #                   ^ first char of label
    #                         ^ (full_label, child)
    is_end:  bool
    value:   Any | None

def make_leaf(value: Any) -> RadixNode:
    return RadixNode(children={}, is_end=True, value=value)

def make_inner() -> RadixNode:
    return RadixNode(children={}, is_end=False, value=None)
```

### Tree

```python
@dataclass
class RadixTree:
    root: RadixNode
    size: int   # number of keys stored
```

### Space Complexity

```
Number of nodes: O(n) where n = number of keys
  (each key creates at most 2 new nodes due to splitting)
Edge labels: total O(sum of key lengths) = O(n · k)
Total space: O(n · k) — same as trie but with a much smaller constant
  (radix tree ≈ 1–5 nodes per key; trie ≈ k nodes per key)
```

## Algorithms (Pure Functions)

```python
def _common_prefix(a: str, b: str) -> str:
    """Return the longest common prefix of strings a and b."""
    i = 0
    while i < len(a) and i < len(b) and a[i] == b[i]:
        i += 1
    return a[:i]

# ─── Search ────────────────────────────────────────────────────────────────

def search(tree: RadixTree, key: str) -> Any | None:
    """
    Return value for key, or None if not found.
    Navigate edge by edge, consuming the matching prefix at each hop.
    Time: O(len(key)).
    """
    return _search_node(tree.root, key)

def _search_node(node: RadixNode, remaining: str) -> Any | None:
    if remaining == "":
        return node.value if node.is_end else None

    first = remaining[0]
    if first not in node.children:
        return None

    label, child = node.children[first]
    prefix = _common_prefix(remaining, label)

    if len(prefix) < len(label):
        # Edge label is longer than what remains — key not in tree
        return None

    # Consumed the full edge label — continue with the rest of the key
    return _search_node(child, remaining[len(label):])

# ─── Starts-with ───────────────────────────────────────────────────────────

def starts_with(tree: RadixTree, prefix: str) -> list[tuple[str, Any]]:
    """
    Return all (key, value) pairs where key starts with prefix.
    Navigate to the prefix node, then collect all entries in its subtree.
    Time: O(len(prefix) + result_chars).
    """
    results: list[tuple[str, Any]] = []
    _starts_with_node(tree.root, prefix, "", results)
    return sorted(results)  # return in lexicographic order

def _starts_with_node(node: RadixNode, remaining_prefix: str,
                      current_key: str, results: list[tuple[str, Any]]) -> None:
    if remaining_prefix == "":
        # We've consumed the entire prefix — collect all keys in this subtree
        _collect_all(node, current_key, results)
        return

    first = remaining_prefix[0]
    if first not in node.children:
        return   # no keys with this prefix

    label, child = node.children[first]
    prefix_match = _common_prefix(remaining_prefix, label)

    if len(prefix_match) == len(remaining_prefix):
        # Prefix is exhausted within this edge — collect from child
        # current_key needs to include the full edge label
        _collect_all_with_label(child, current_key + label, results)
    elif len(prefix_match) == len(label):
        # Consumed the full edge — continue matching prefix in child
        _starts_with_node(child, remaining_prefix[len(label):],
                          current_key + label, results)
    # else: prefix diverges from label — no match

def _collect_all(node: RadixNode, current: str, results: list[tuple[str, Any]]) -> None:
    """DFS: collect all (key, value) in subtree rooted at node."""
    if node.is_end:
        results.append((current, node.value))
    for first_char in sorted(node.children):
        label, child = node.children[first_char]
        _collect_all(child, current + label, results)

def _collect_all_with_label(node: RadixNode, accumulated: str,
                             results: list[tuple[str, Any]]) -> None:
    """Same as _collect_all but the edge label has already been added."""
    _collect_all(node, accumulated, results)

# ─── Longest prefix match ──────────────────────────────────────────────────

def longest_prefix_match(tree: RadixTree, string: str) -> tuple[str, Any] | None:
    """
    Return (key, value) where key is the longest stored key that is a
    prefix of string. Returns None if no stored key is a prefix of string.
    Time: O(len(string)).

    Example: stored keys ["10", "10.0", "10.0.0"], string "10.0.0.1"
    → returns ("10.0.0", value)
    """
    return _lpm_node(tree.root, string, "", None)

def _lpm_node(node: RadixNode, remaining: str, current: str,
              best: tuple[str, Any] | None) -> tuple[str, Any] | None:
    if node.is_end:
        best = (current, node.value)   # this is a longer match than previous best
    if remaining == "":
        return best

    first = remaining[0]
    if first not in node.children:
        return best

    label, child = node.children[first]
    prefix = _common_prefix(remaining, label)

    if len(prefix) < len(label):
        # Partial edge match — the string runs out or diverges mid-edge
        # The current 'best' is the answer
        return best

    return _lpm_node(child, remaining[len(label):], current + label, best)

# ─── Insert ────────────────────────────────────────────────────────────────

def insert(tree: RadixTree, key: str, value: Any) -> RadixTree:
    """
    Return new RadixTree with key→value inserted.
    Handles all 4 insertion cases via edge splitting.
    Time: O(len(key)).
    """
    already_exists = search(tree, key) is not None
    new_root = _insert_node(tree.root, key, value)
    return RadixTree(
        root=new_root,
        size=tree.size + (0 if already_exists else 1),
    )

def _insert_node(node: RadixNode, key: str, value: Any) -> RadixNode:
    """Insert key into node's subtree. Return new (possibly different) node."""
    if key == "":
        # Key ends at this node
        return RadixNode(children=dict(node.children), is_end=True, value=value)

    first = key[0]
    children = dict(node.children)

    if first not in children:
        # Case 1: No common prefix — add new leaf edge
        children[first] = (key, make_leaf(value))
        return RadixNode(children=children, is_end=node.is_end, value=node.value)

    label, child = children[first]
    prefix = _common_prefix(key, label)
    p = len(prefix)

    if p == len(label) and p == len(key):
        # Exact match with existing edge — update value at child
        new_child = RadixNode(
            children=dict(child.children), is_end=True, value=value
        )
        children[first] = (label, new_child)

    elif p == len(label):
        # Case 2: Edge label is a prefix of new key — descend
        new_child = _insert_node(child, key[p:], value)
        children[first] = (label, new_child)

    elif p == len(key):
        # Case 3: New key is a prefix of edge label — split, new key ends at split
        remaining_label = label[p:]   # e.g., label="apple", key="app" → remaining="le"
        split_node = RadixNode(
            children={remaining_label[0]: (remaining_label, child)},
            is_end=True,
            value=value,
        )
        children[first] = (key, split_node)

    else:
        # Case 4: Partial match — split at divergence point
        remaining_key   = key[p:]    # rest of new key after shared prefix
        remaining_label = label[p:]  # rest of old label after shared prefix
        split_node = RadixNode(
            children={
                remaining_label[0]: (remaining_label, child),
                remaining_key[0]:   (remaining_key, make_leaf(value)),
            },
            is_end=False,
            value=None,
        )
        children[first] = (prefix, split_node)

    return RadixNode(children=children, is_end=node.is_end, value=node.value)

# ─── Delete ────────────────────────────────────────────────────────────────

def delete(tree: RadixTree, key: str) -> RadixTree:
    """
    Return new RadixTree with key removed.
    Merges nodes that become collapsible after deletion.
    Time: O(len(key)).
    """
    if search(tree, key) is None:
        return tree
    new_root, _ = _delete_node(tree.root, key)
    return RadixTree(root=new_root or make_inner(), size=tree.size - 1)

def _delete_node(node: RadixNode, remaining: str) -> tuple[RadixNode | None, bool]:
    """
    Remove remaining key from node's subtree.
    Returns (new_node, was_deleted).
    new_node is None if this node should be removed entirely.
    """
    if remaining == "":
        if not node.is_end:
            return node, False   # key not here
        if not node.children:
            return None, True    # leaf with no children — remove entirely
        return RadixNode(children=dict(node.children), is_end=False, value=None), True

    first = remaining[0]
    if first not in node.children:
        return node, False

    label, child = node.children[first]
    prefix = _common_prefix(remaining, label)
    if len(prefix) < len(label):
        return node, False   # key not in tree

    new_child, deleted = _delete_node(child, remaining[len(label):])
    if not deleted:
        return node, False

    children = dict(node.children)
    if new_child is None:
        del children[first]
    else:
        # Check if we can merge: new_child has exactly one child and is not an endpoint
        if not new_child.is_end and len(new_child.children) == 1:
            # Merge new_child with its only child
            only_char = next(iter(new_child.children))
            child_label, grandchild = new_child.children[only_char]
            merged_label = label + child_label
            children[first] = (merged_label, grandchild)
        else:
            children[first] = (label, new_child)

    new_node = RadixNode(children=children, is_end=node.is_end, value=node.value)
    if not new_node.is_end and len(new_node.children) == 0:
        return None, True   # this node is now unused
    return new_node, True

# ─── All entries ───────────────────────────────────────────────────────────

def all_entries(tree: RadixTree) -> list[tuple[str, Any]]:
    """Return all (key, value) pairs in lexicographic order. O(n · k)."""
    results: list[tuple[str, Any]] = []
    _collect_all(tree.root, "", results)
    return results
```

## Public API

```python
from typing import Any, Generic, TypeVar, Iterator

V = TypeVar("V")

class RadixTree(Generic[V]):
    """
    A radix tree (compressed trie) mapping string keys to values.

    Memory-efficient alternative to a Trie (DT13): chains of single-child
    nodes are collapsed into single edges with multi-character labels.

    Supports the same prefix operations as Trie but with fewer nodes:
      - O(n) nodes instead of O(n·k) nodes
      - Each key stored using at most 2 new nodes (worst case: one split)

    Real-world analogy: Redis rax.c uses this structure for all key storage.
    HTTP routers (gorilla/mux, actix-web) use radix trees for URL pattern matching.
    """

    def __init__(self) -> None: ...

    # ─── Core operations ─────────────────────────────────────────────
    def insert(self, key: str, value: V) -> None:
        """
        Store key→value. Handles all 4 insertion cases.
        May split an existing edge. O(len(key)).
        """
        ...

    def search(self, key: str) -> V | None:
        """
        Exact match. Return value if found, None otherwise.
        O(len(key)).
        """
        ...

    def delete(self, key: str) -> None:
        """
        Remove key. Merges collapsible nodes after deletion.
        No-op if key not present. O(len(key)).
        """
        ...

    def __contains__(self, key: str) -> bool: ...
    def __getitem__(self, key: str) -> V: ...    # raises KeyError
    def __setitem__(self, key: str, value: V) -> None: ...
    def __delitem__(self, key: str) -> None: ...  # raises KeyError

    # ─── Prefix operations ───────────────────────────────────────────
    def starts_with(self, prefix: str) -> list[tuple[str, V]]:
        """
        Return all (key, value) pairs where key starts with prefix.
        O(len(prefix) + total_result_chars).
        """
        ...

    def longest_prefix_match(self, string: str) -> tuple[str, V] | None:
        """
        Return (key, value) for the longest stored key that is a prefix
        of string. Returns None if no stored key is a prefix.
        O(len(string)).
        Classic use: IP routing (longest matching network prefix).
        """
        ...

    # ─── Iteration ───────────────────────────────────────────────────
    def all_entries(self) -> list[tuple[str, V]]:
        """All (key, value) in lexicographic order. O(n · k)."""
        ...

    def __iter__(self) -> Iterator[str]:
        """Iterate all keys in lexicographic order."""
        ...

    def items(self) -> Iterator[tuple[str, V]]:
        """Iterate (key, value) pairs in lexicographic order."""
        ...

    # ─── Metadata ────────────────────────────────────────────────────
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...

    def node_count(self) -> int:
        """
        Count total nodes in the tree. Useful for measuring compression ratio.
        Compare to len(self) * average_key_length to see memory savings vs trie.
        O(n).
        """
        ...

    def is_valid(self) -> bool:
        """
        Verify structural invariants:
          - No node has one child and is_end=False (should have been merged).
          - Each child's edge label starts with the key used to index it in children dict.
          - All is_end nodes are reachable and their accumulated key gives correct search.
          - size matches actual is_end count.
        O(n · k). For testing only.
        """
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

RadixTree implements the same interface as Trie (DT13). Swap them transparently.

```python
# Python — implements the same SearchTree interface as Trie
class RadixTree(Generic[V]):
    def __init__(self):
        self._root = RadixNode(children={}, is_end=False, value=None)
        self._size = 0

    # Implements: insert, search, delete, starts_with, longest_prefix_match,
    #             all_entries, __iter__, items, __len__, __contains__
```

```typescript
// TypeScript
interface PrefixTree<V> {
  insert(key: string, value: V): void;
  search(key: string): V | undefined;
  delete(key: string): void;
  startsWith(prefix: string): Array<[string, V]>;
  longestPrefixMatch(s: string): [string, V] | undefined;
}

class RadixTree<V> implements PrefixTree<V> {
  private root: RadixNode<V> = new RadixNode();
  private _size = 0;
}

class RadixNode<V> {
  children = new Map<string, [string, RadixNode<V>]>();  // firstChar → (label, child)
  isEnd = false;
  value: V | undefined;
}
```

```ruby
# Ruby
class RadixTree
  include Enumerable

  def initialize
    @root = RadixNode.new
    @size = 0
  end

  def each(&block)
    collect_all(@root, "", &block)
  end

  private

  RadixNode = Struct.new(:children, :is_end, :value) do
    def initialize = super({}, false, nil)
  end
end
```

### Composition (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — edge labels stored as Strings; children keyed by first byte
use std::collections::HashMap;

pub struct RadixTree<V> {
    root: RadixNode<V>,
    size: usize,
}

struct RadixNode<V> {
    // Key: first char of edge label (for O(1) child lookup)
    // Value: (full edge label, child node)
    children: HashMap<char, (String, Box<RadixNode<V>>)>,
    is_end:   bool,
    value:    Option<V>,
}

impl<V: Clone> RadixTree<V> {
    pub fn new() -> Self {
        Self { root: RadixNode::new(), size: 0 }
    }

    pub fn insert(&mut self, key: &str, value: V) { ... }
    pub fn search(&self, key: &str) -> Option<&V> { ... }
    pub fn delete(&mut self, key: &str) -> bool { ... }
    pub fn starts_with(&self, prefix: &str) -> Vec<(String, &V)> { ... }
    pub fn longest_prefix_match(&self, s: &str) -> Option<(String, &V)> { ... }
}
```

```go
// Go
type RadixTree[V any] struct {
    root *radixNode[V]
    size int
}

type radixNode[V any] struct {
    // map from first char of edge label → (full label, child)
    children map[rune]radixEdge[V]
    isEnd    bool
    value    V
    hasValue bool
}

type radixEdge[V any] struct {
    label string
    child *radixNode[V]
}
```

```elixir
# Elixir — immutable persistent radix tree
defmodule RadixTree do
  # node = %{children: %{first_char => {label, node}}, is_end: bool, value: any}
  def new(), do: %{children: %{}, is_end: false, value: nil}

  def insert(tree, key, value) do
    %{tree | root: insert_node(tree.root, key, value)}
  end

  defp insert_node(node, "", value) do
    %{node | is_end: true, value: value}
  end

  defp insert_node(node, key, value) do
    first = String.first(key)
    case Map.get(node.children, first) do
      nil ->
        leaf = %{children: %{}, is_end: true, value: value}
        %{node | children: Map.put(node.children, first, {key, leaf})}
      {label, child} ->
        prefix = common_prefix(key, label)
        # ... handle 4 cases
    end
  end

  defp common_prefix(a, b) do
    Enum.zip(String.graphemes(a), String.graphemes(b))
    |> Enum.take_while(fn {x, y} -> x == y end)
    |> Enum.map(&elem(&1, 0))
    |> Enum.join()
  end
end
```

## Test Strategy

### Invariant Verifier

```python
def verify_radix(tree: RadixTree) -> None:
    """
    Assert all radix tree invariants.
    Call after every insert and delete in tests.
    """
    actual_size = [0]
    _verify_node(tree.root, "", actual_size)
    assert actual_size[0] == len(tree), f"size mismatch: {len(tree)} vs {actual_size[0]}"

def _verify_node(node: RadixNode, accumulated_key: str, count: list[int]) -> None:
    if node.is_end:
        count[0] += 1

    # A non-end inner node with exactly one child should have been merged
    if not node.is_end and len(node.children) == 1:
        # This might be the root (special case) — check parent context
        # In non-root position, this is a violation
        pass  # handled by caller

    for first_char, (label, child) in node.children.items():
        assert label[0] == first_char, f"Edge indexed by '{first_char}' but label starts with '{label[0]}'"
        assert len(label) > 0, "Edge label must be non-empty"
        _verify_node(child, accumulated_key + label, count)
```

### Test Cases

```
1. Empty tree: search("x") → None, all_entries() → [], len == 0.

2. Single insert: insert("hello", 42).
   search("hello") → 42. search("hell") → None. search("hellos") → None.

3. Sequential inserts (Case 2 — extending existing edge):
   insert "app", then insert "apple".
   After "apple": root → "app"(*) → "le"(*)
   search("app") → value. search("apple") → value.

4. Case 3 — new key is prefix of existing:
   insert "apple", then insert "app".
   After: root → "app"(*) → "le"(*)
   node_count() should be 2 (root + split_node + leaf = 3 total,
   but root is transparent).
   verify_radix() must pass.

5. Case 4 — partial match, split:
   insert "application", then insert "apple".
   After: root → "appl" → "ication"(*) and "e"(*)
   search("application") → value. search("apple") → value.
   search("appl") → None.

6. starts_with("app"):
   insert ["apple", "application", "apply", "apt", "banana"].
   starts_with("app") → [("apple",..), ("application",..), ("apply",..)].
   starts_with("ban") → [("banana",..)].
   starts_with("xyz") → [].

7. longest_prefix_match — IP routing simulation:
   insert ["10", "10.0", "10.0.0", "192", "192.168"].
   longest_prefix_match("10.0.0.1") → ("10.0.0", ..)
   longest_prefix_match("10.0.1.5") → ("10.0", ..)
   longest_prefix_match("10.5.0.1") → ("10", ..)
   longest_prefix_match("172.0.0.1") → None

8. Delete leaf:
   insert ["app", "apple"], delete "apple".
   search("apple") → None. search("app") → value.
   Node at "app" should not have been merged back (it's still is_end=True).
   verify_radix() passes.

9. Delete non-leaf with merge:
   insert ["apple"], delete has no merge.
   insert ["app", "apple"], delete "app".
   Now "app" node: is_end=False, one child "le".
   Should merge: root → "apple"(*) (one edge, no intermediate node).
   node_count() == 1.

10. Delete with propagating merge:
    insert ["abc"], delete "abc".
    Tree should be empty (root has no children, is_end=False).

11. Update existing key: insert("key", "v1"), insert("key", "v2").
    search("key") → "v2". len still 1.

12. all_entries sorted:
    insert in random order [banana, apple, cherry, apricot].
    all_entries() → sorted list.

13. node_count compression ratio:
    insert 100 keys all starting with "prefix_" (7 chars shared).
    Verify node_count() << 100 * 7 (compression ratio ≥ 5x vs trie).

14. Equivalence with Trie:
    Insert the same 1000 keys into both a Trie and a RadixTree.
    Verify: search, starts_with, longest_prefix_match, all_entries
    return identical results for all queries.

15. Unicode keys: insert ["日本語", "日本", "日"], verify all three searchable.
    starts_with("日本") → [("日本",..),("日本語",..)].
```

### Coverage Targets

- 95%+ line coverage
- All 4 insertion cases (no match, extend, key-is-prefix, partial-match)
- Delete: leaf removal, node merge after deletion, cascading merge
- starts_with: empty prefix (all entries), exact match, partial match, no match
- longest_prefix_match: no match, exact match, mid-edge match, multiple candidates
- Edge labels of length 1, 2, and 10+ characters
- Trees of height 1, 3, and 10+ levels

## Future Extensions

- **DT15 Suffix tree** — build a radix tree over all suffixes of a string.
  The Ukkonen algorithm constructs it in O(n) time. Enables O(m) substring search
  in a text of length n (instead of O(n·m) for naive search or O(n) for KMP).
  Bioinformatics uses suffix trees for DNA sequence alignment.
- **Bitwise radix tree (Patricia trie)** — use single bits as edge labels.
  Each node branches on one bit of the key. For 32-bit integers or IPv4 addresses,
  the tree is at most 32 levels deep. Faster than character-level for fixed-width keys.
- **Adaptive Radix Tree (ART)** — used in main-memory databases. Nodes adapt their
  fan-out to the number of children: node4 (≤4 children), node16, node48, node256.
  Reduces memory vs full 256-way branching while keeping O(1) child lookup.
  VoltDB, HyPer, and Umbra use ART as their primary index.
- **HAMT (Hash Array Mapped Trie)** — applies radix tree compression to hash values.
  Keys are hashed; the hash is used as the bit-string in a bitwise trie. Enables
  O(log n) operations with excellent cache performance. Used in Clojure, Scala,
  and Haskell as the persistent map implementation.
- **Concurrent radix tree** — lock-free reads with RCU (read-copy-update). Writers
  copy the path from root to the modified node, update the copy, then atomically
  swap the root pointer. Readers never block. Used in Linux kernel's radix tree
  for page cache management.
