# DT13 — Trie (Prefix Tree)

## Overview

A **trie** (pronounced "try", from the middle syllable of re**trie**val) is a tree
where each path from the root to a node spells out a string prefix. Unlike a hash
map or BST that treats keys as opaque blobs, a trie decomposes each key into
individual characters and routes through one tree level per character.

The payoff: **prefix operations become trivial**. "Find all words starting with
'app'" in a hash map means scanning every key — O(n·k) where k is average key
length. In a trie, you navigate to the 'app' node in O(p) time (p = prefix length)
and then collect all words in that subtree. Autocomplete, spell-check, IP routing
tables, and DNA sequence indexing all rely on this property.

## Layer Position

```
DT00: graph
DT02: tree
DT13: trie          ← [YOU ARE HERE] (string-keyed tree)
  └── DT14: radix-tree  (compressed trie: chains of single-child nodes collapsed)
        └── DT15: suffix-tree  (radix tree of all suffixes of a string)

DT11: b-tree        (sibling: key-comparison tree for sorted data)
DT18: hash-map      (sibling: O(1) point lookup but no prefix operations)
```

**Depends on:** DT02 (tree: nodes, parent-child relationships, DFS).
**Used by:** DT14 (radix tree compresses tries), DT15 (suffix tree).
**Complementary to:** DT18 (hash-map): use a hash-map when you need fast exact
lookup and don't care about prefixes; use a trie when prefix queries matter.

## Concepts

### The Fundamental Idea

Imagine you have a dictionary of words. In a hash map, each word is hashed to a
bucket — completely independent of other words. There is no structural relationship
between "apple", "app", and "application" even though they share a prefix.

A trie makes that shared prefix explicit by sharing tree nodes:

```
Words: ["apple", "app", "application", "apply", "apt", "banana"]

        (root)
       /       \
      a          b
      |          |
      p          a
     / \         |
    p   t        n
    |   |        |
   (*)  (*)      a
   /              \
  l               n
 / \               \
e   i               a
|   |              (*)
(*) c
    |
    a
    |
    t
    |
    i
    |
    o
    |
    n
   (*)

(*) marks nodes where a complete word terminates.

Read paths from root:
  root→a→p→p→(*) = "app"          ✓
  root→a→p→p→l→e→(*) = "apple"    ✓
  root→a→p→p→l→i→c→a→t→i→o→n→(*) = "application"  ✓
  root→a→p→p→l→y→(*) = "apply"    ✓
  root→a→p→t→(*) = "apt"          ✓
  root→b→a→n→a→n→a→(*) = "banana" ✓
```

The critical observation: "app", "apple", "application", and "apply" all share
the path `root→a→p→p`. The trie physically shares these 3 nodes among 4 words.
In a hash map, those 4 words have no structural relationship at all.

### Node Structure: Two Designs

**Design 1 — Array of 26 children (for lowercase letters):**

```
class TrieNode:
    children: list[TrieNode | None]  # length 26, one slot per letter a–z
    is_end:   bool
    value:    Any | None  # if storing values (not just words)

Access: node.children[ord(c) - ord('a')]
```

```
Pros:
  O(1) child lookup (direct array index)
  Cache-friendly (array is contiguous memory)
  Simple implementation

Cons:
  26 pointers per node even if node has only 1-2 children
  For a trie with 10,000 nodes: 10,000 × 26 × 8 bytes = 2 MB for pointers alone
  Wastes memory for sparse alphabets (e.g., only 5 distinct characters used)
  Doesn't generalize to Unicode or arbitrary keys
```

**Design 2 — Map of children (dict/HashMap):**

```
class TrieNode:
    children: dict[str, TrieNode]  # char → child node
    is_end:   bool
    value:    Any | None
```

```
Pros:
  O(1) average lookup (hash map)
  Memory-efficient: only stores children that exist
  Generalizes to any character set (Unicode, bytes, DNA bases)
  With 5-character average key: same 10,000 nodes use ~5× less memory

Cons:
  Hash map overhead per node
  Cache-unfriendly (hash map entries scattered in memory)
```

**Practical recommendation:** use a map. The alphabet is rarely dense enough to
justify the 26-array approach, and maps generalize to any key type.

### Search

```
Search for "apple" in the trie:

Start at root. Process each character:
  'a': root.children['a'] exists? YES → move to node_a
  'p': node_a.children['p'] exists? YES → move to node_ap
  'p': node_ap.children['p'] exists? YES → move to node_app
  'l': node_app.children['l'] exists? YES → move to node_appl
  'e': node_appl.children['e'] exists? YES → move to node_apple

Reached end of string. Is node_apple.is_end == True? YES → found!

Search for "applying":
  Navigate a→p→p→l→y→(is_end=True at 'apply' node)...
  'i': node_apply.children['i'] exists? NO → return None
```

The key distinction: `starts_with("app")` only needs to reach the 'app' node and
return True. `search("app")` additionally checks `is_end` at that node.

### Insertion

```
Insert "apt" into a trie that already has "apple", "app":

Existing path: root→a→p→p→l→e(*)

Insert "apt":
  'a': root.children['a'] exists → follow
  'p': node_a.children['p'] exists → follow
  't': node_ap.children['t'] exists? NO → create new node_t
  Mark node_t.is_end = True

After insert:
  root→a→p→p→l→e(*)     ← existing path unchanged
          \
           t(*)           ← new branch from node_ap

Cost: O(len(word)) — one node creation per new character at most.
```

### Prefix Search (Autocomplete)

```
Find all words starting with "app":

Step 1: Navigate to the "app" node.
  root → a → p → p
  If this path doesn't exist, return [].

Step 2: DFS from the "app" node, collecting all is_end nodes.

DFS from node_app, accumulating suffix "":
  At node_app: is_end=True? YES → emit "app"
  Visit child 'l' (suffix "l"):
    Visit child 'e' (suffix "le"):
      is_end=True? YES → emit "apple"
    Visit child 'i' (suffix "li"):
      Visit child 'c' → 'a' → 't' → 'i' → 'o' → 'n':
        is_end=True? YES → emit "application"
    Visit child 'y' (suffix "ly"):
      is_end=True? YES → emit "apply"

Result: ["app", "apple", "application", "apply"]
```

This is O(p + r · k) where:
- p = prefix length (navigating to the node)
- r = number of results
- k = average result length (collecting each word)

Compare to hash map: O(n · k) — scan all n keys of average length k.
For a dictionary of 100,000 words with prefix "app" returning 50 results:
```
Trie:     O(3) + O(50 × 7) = ~353 operations
Hash map: O(100,000 × 7)  = ~700,000 operations
```

### Longest Prefix Match

A special operation used in IP routing tables: given a string, find the longest
stored word that is a prefix of the input.

```
Trie contains: ["a", "ab", "abc", "abcd", "xyz"]
Query: "abcdef"

Walk character by character, tracking the last seen is_end:
  'a': node exists, is_end=True → last_match = "a"
  'b': node exists, is_end=True → last_match = "ab"
  'c': node exists, is_end=True → last_match = "abc"
  'd': node exists, is_end=True → last_match = "abcd"
  'e': node does NOT exist → stop

Return: "abcd"
```

IP routers store route prefixes (e.g., "10.0.0", "10.0.1", "192.168") in a
binary trie (one node per bit). For each incoming packet, they find the longest
matching prefix to determine the outgoing interface. This runs millions of times
per second — the trie's O(k) worst case (k = key length) is key.

### Deletion

```
Delete "app" from trie containing "app" and "apple":

Navigate to the "app" node. It has is_end=True.

Can we delete the node itself? NO — it has children ('l' branch for "apple").
Strategy: just set is_end=False.

After deletion:
  root→a→p→p(is_end=False)→l→e(*)
  "apple" still exists. "app" no longer exists.

Now delete "apple" (no other words share its unique suffix "le"):
  Navigate to node_apple.
  It's a leaf (no children) and is_end=True.
  Safe to delete the node.
  After deleting node_apple, node_appl is also a leaf with is_end=False.
  Safe to delete node_appl too.
  After deleting node_appl, node_app now has no children. is_end=False.
  Safe to delete node_app.
  After deleting node_app, node_ap has one child left (if 'apt' exists) or none.

General rule:
  After removing is_end, walk back up and delete nodes that have:
    - is_end=False (no word ends here)
    - no children (no word passes through here)
```

### Worked Example: Full Trie with Values

Tries can store key-value pairs (like a dict) instead of just words.

```
Store phone book: {
  "alice": "555-1234",
  "ali":   "555-0001",
  "bob":   "555-5678",
  "bobby": "555-9999",
}

Trie:
  root
  ├── a
  │   └── l
  │       ├── i (value="555-0001", is_end=True)
  │       │   └── c
  │       │       └── e (value="555-1234", is_end=True)
  └── b
      └── o
          └── b (value="555-5678", is_end=True)
              └── b
                  └── y (value="555-9999", is_end=True)

search("ali")   → "555-0001"
search("alice") → "555-1234"
search("al")    → None (no is_end at 'l' node)
words_with_prefix("b") → [("bob","555-5678"), ("bobby","555-9999")]
```

## Representation

### Node

```python
@dataclass
class TrieNode:
    children: dict[str, "TrieNode"]  # char → child
    is_end:   bool                   # True if a complete key ends here
    value:    Any | None             # stored value (None for word-only tries)
```

### Tree

```python
@dataclass
class Trie:
    root:       TrieNode
    word_count: int   # number of complete keys stored
```

### Space Complexity

```
Worst case (no shared prefixes): O(n · k) nodes
  where n = number of keys, k = average key length.

Best case (long common prefix): O(n + L) nodes
  where L = total length of all distinct prefixes.

Typical case: much better than worst case. English words share many prefixes
("pre-", "un-", "-tion", "-ing"). A dictionary trie is usually 3-5× smaller
than a flat array of all words.
```

## Algorithms (Pure Functions)

```python
# ─── Insert ────────────────────────────────────────────────────────────────

def insert(trie: Trie, key: str, value: Any = True) -> Trie:
    """
    Return new trie with key→value stored.
    Creates nodes for each character in key that don't already exist.
    If key already exists, updates its value.
    Time: O(len(key)).
    """
    new_root = _insert_node(trie.root, key, value, 0)
    old_exists = search(trie, key) is not None
    return Trie(
        root=new_root,
        word_count=trie.word_count + (0 if old_exists else 1),
    )

def _insert_node(node: TrieNode, key: str, value: Any, depth: int) -> TrieNode:
    if depth == len(key):
        # Reached end of key — mark this node as a word endpoint
        return TrieNode(
            children=dict(node.children),
            is_end=True,
            value=value,
        )
    c = key[depth]
    children = dict(node.children)
    child = children.get(c, TrieNode({}, False, None))
    children[c] = _insert_node(child, key, value, depth + 1)
    return TrieNode(children=children, is_end=node.is_end, value=node.value)

# ─── Search ────────────────────────────────────────────────────────────────

def search(trie: Trie, key: str) -> Any | None:
    """
    Exact match lookup. Returns value if key exists, None otherwise.
    'app' does NOT match if only 'apple' is stored.
    Time: O(len(key)).
    """
    node = _find_node(trie.root, key)
    if node is None or not node.is_end:
        return None
    return node.value

def _find_node(node: TrieNode | None, key: str) -> TrieNode | None:
    """Navigate to the node at the end of key's path, or None."""
    for c in key:
        if node is None or c not in node.children:
            return None
        node = node.children[c]
    return node

# ─── Prefix check ──────────────────────────────────────────────────────────

def starts_with(trie: Trie, prefix: str) -> bool:
    """
    Return True if any stored key starts with prefix.
    Time: O(len(prefix)).
    """
    return _find_node(trie.root, prefix) is not None

# ─── Autocomplete ──────────────────────────────────────────────────────────

def words_with_prefix(trie: Trie, prefix: str) -> list[tuple[str, Any]]:
    """
    Return all (key, value) pairs where key starts with prefix.
    Time: O(len(prefix) + result_chars) where result_chars = sum of lengths of results.
    """
    node = _find_node(trie.root, prefix)
    if node is None:
        return []
    results: list[tuple[str, Any]] = []
    _collect_all(node, prefix, results)
    return results

def _collect_all(node: TrieNode, current: str, results: list[tuple[str, Any]]) -> None:
    """DFS collecting all complete words in the subtree rooted at node."""
    if node.is_end:
        results.append((current, node.value))
    for c, child in sorted(node.children.items()):  # sorted for deterministic order
        _collect_all(child, current + c, results)

# ─── All words ──────────────────────────────────────────────────────────────

def all_words(trie: Trie) -> list[tuple[str, Any]]:
    """
    Return all (key, value) pairs stored in the trie, in lexicographic order.
    Time: O(n · k) where n = number of words, k = average length.
    Equivalent to words_with_prefix(trie, "").
    """
    return words_with_prefix(trie, "")

# ─── Longest prefix match ──────────────────────────────────────────────────

def longest_prefix_match(trie: Trie, string: str) -> tuple[str, Any] | None:
    """
    Return the (key, value) pair where key is the longest stored word that is
    a prefix of string. Returns None if no stored word is a prefix of string.

    Example: trie has ["a", "ab", "abc"]; string = "abcdef"
    → returns ("abc", value_of_abc)

    Use case: IP routing, command parsing, URL routing.
    Time: O(len(string)).
    """
    node = trie.root
    last_match: tuple[str, Any] | None = None
    current = []

    for c in string:
        if c not in node.children:
            break
        node = node.children[c]
        current.append(c)
        if node.is_end:
            last_match = ("".join(current), node.value)

    return last_match

# ─── Delete ────────────────────────────────────────────────────────────────

def delete(trie: Trie, key: str) -> Trie:
    """
    Return new trie with key removed.
    Cleans up nodes that become unreachable (no children, no is_end).
    Time: O(len(key)).
    """
    if search(trie, key) is None:
        return trie   # key not present; no-op
    new_root = _delete_node(trie.root, key, 0)
    return Trie(root=new_root or TrieNode({}, False, None),
                word_count=trie.word_count - 1)

def _delete_node(node: TrieNode | None, key: str, depth: int) -> TrieNode | None:
    """
    Return new node after deleting key[depth:].
    Returns None if this node should be removed (no children, not an endpoint).
    """
    if node is None:
        return None
    if depth == len(key):
        # Found the end of the key — unmark it
        if not node.children:
            return None  # leaf node with no other purpose → delete it
        return TrieNode(children=node.children, is_end=False, value=None)
    c = key[depth]
    if c not in node.children:
        return node  # key not found (shouldn't happen if we checked above)
    children = dict(node.children)
    new_child = _delete_node(children[c], key, depth + 1)
    if new_child is None:
        del children[c]
    else:
        children[c] = new_child
    if not children and not node.is_end:
        return None  # this node is now useless → let parent remove it
    return TrieNode(children=children, is_end=node.is_end, value=node.value)

# ─── Word count ────────────────────────────────────────────────────────────

def word_count(trie: Trie) -> int:
    """Return number of keys stored. O(1)."""
    return trie.word_count
```

## Public API

```python
from typing import Any, Generic, TypeVar, Iterator

V = TypeVar("V")

class Trie(Generic[V]):
    """
    A prefix tree mapping string keys to values.

    Unlike a dict, a Trie supports prefix operations:
      - Find all keys starting with a prefix (autocomplete)
      - Check if any key starts with a prefix
      - Find the longest key that is a prefix of a given string (IP routing)

    Keys must be strings (or byte sequences). Values are arbitrary.
    If you only need to store words (no values), use V=bool and value=True.
    """

    def __init__(self) -> None: ...

    # ─── Core operations ─────────────────────────────────────────────
    def insert(self, key: str, value: V = True) -> None:
        """
        Store key→value. If key exists, update value.
        Time: O(len(key)).
        """
        ...

    def search(self, key: str) -> V | None:
        """
        Exact match: return value if key exists, None otherwise.
        'app' returns None if only 'apple' is stored.
        Time: O(len(key)).
        """
        ...

    def delete(self, key: str) -> None:
        """
        Remove key. No-op if key not present.
        Cleans up now-unused nodes.
        Time: O(len(key)).
        """
        ...

    def __contains__(self, key: str) -> bool:
        """key in trie → bool. O(len(key))."""
        ...

    def __getitem__(self, key: str) -> V:
        """trie[key] → raises KeyError if not found."""
        ...

    def __setitem__(self, key: str, value: V) -> None:
        """trie[key] = value."""
        ...

    def __delitem__(self, key: str) -> None:
        """del trie[key] → raises KeyError if not found."""
        ...

    # ─── Prefix operations ───────────────────────────────────────────
    def starts_with(self, prefix: str) -> bool:
        """
        Return True if any stored key starts with prefix.
        Time: O(len(prefix)).
        """
        ...

    def words_with_prefix(self, prefix: str) -> list[tuple[str, V]]:
        """
        Return all (key, value) pairs where key starts with prefix.
        Returned in lexicographic order.
        Time: O(len(prefix) + total_result_chars).
        """
        ...

    def longest_prefix_match(self, string: str) -> tuple[str, V] | None:
        """
        Return (key, value) where key is the longest stored key that
        is a prefix of string. Returns None if no match.
        Time: O(len(string)).
        Example uses: IP routing, URL dispatch, command parsing.
        """
        ...

    # ─── Iteration ───────────────────────────────────────────────────
    def all_words(self) -> list[tuple[str, V]]:
        """All (key, value) pairs in lexicographic order. O(n · k)."""
        ...

    def __iter__(self) -> Iterator[str]:
        """Iterate all keys in lexicographic order."""
        ...

    def items(self) -> Iterator[tuple[str, V]]:
        """Iterate (key, value) pairs in lexicographic order."""
        ...

    # ─── Metadata ────────────────────────────────────────────────────
    def __len__(self) -> int:
        """Number of keys stored. O(1)."""
        ...

    def __bool__(self) -> bool:
        """True if any key is stored."""
        ...

    def is_valid(self) -> bool:
        """
        Verify structural invariants. For testing only.
        Checks: word_count matches actual is_end count, no null nodes.
        O(n · k).
        """
        ...
```

## Composition Model

### Inheritance (Python, Ruby, TypeScript)

Trie is self-contained — it doesn't inherit from Graph, Tree, or BST. Define
a concrete class directly.

```python
# Python
class Trie(Generic[V]):
    def __init__(self):
        self._root = _TrieNode()
        self._size = 0

@dataclass
class _TrieNode:
    children: dict[str, "_TrieNode"] = field(default_factory=dict)
    is_end:   bool = False
    value:    Any  = None
```

```typescript
// TypeScript
class Trie<V> {
  private root: TrieNode<V> = new TrieNode();
  private _size = 0;

  insert(key: string, value: V): void { ... }
  search(key: string): V | undefined { ... }
  wordsWithPrefix(prefix: string): Array<[string, V]> { ... }
}

class TrieNode<V> {
  children = new Map<string, TrieNode<V>>();
  isEnd = false;
  value: V | undefined = undefined;
}
```

```ruby
# Ruby
class Trie
  include Enumerable

  def initialize
    @root = TrieNode.new
    @size = 0
  end

  def each(&block)
    collect_all(@root, "", &block)
  end

  private

  TrieNode = Struct.new(:children, :is_end, :value) do
    def initialize = super({}, false, nil)
  end
end
```

### Composition (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — generic over value type
use std::collections::HashMap;

pub struct Trie<V> {
    root: TrieNode<V>,
    size: usize,
}

struct TrieNode<V> {
    children: HashMap<char, TrieNode<V>>,
    is_end:   bool,
    value:    Option<V>,
}

impl<V: Clone> Trie<V> {
    pub fn new() -> Self {
        Self { root: TrieNode::new(), size: 0 }
    }

    pub fn insert(&mut self, key: &str, value: V) { ... }
    pub fn search(&self, key: &str) -> Option<&V> { ... }
    pub fn words_with_prefix(&self, prefix: &str) -> Vec<(String, &V)> { ... }
    pub fn longest_prefix_match(&self, s: &str) -> Option<(String, &V)> { ... }
}
```

```go
// Go
type Trie[V any] struct {
    root *trieNode[V]
    size int
}

type trieNode[V any] struct {
    children map[rune]*trieNode[V]
    isEnd    bool
    value    V
    hasValue bool
}

func NewTrie[V any]() *Trie[V] {
    return &Trie[V]{root: &trieNode[V]{children: make(map[rune]*trieNode[V])}}
}
```

```elixir
# Elixir — immutable trie as nested maps
defmodule Trie do
  # node = %{children: %{char => node}, is_end: bool, value: any}
  def new(), do: %{children: %{}, is_end: false, value: nil}

  def insert(trie, key, value \\ true) do
    chars = String.graphemes(key)
    insert_node(trie, chars, value)
  end

  defp insert_node(node, [], value) do
    %{node | is_end: true, value: value}
  end
  defp insert_node(node, [c | rest], value) do
    child = Map.get(node.children, c, new())
    new_child = insert_node(child, rest, value)
    %{node | children: Map.put(node.children, c, new_child)}
  end

  def search(trie, key) do
    chars = String.graphemes(key)
    case find_node(trie, chars) do
      nil -> nil
      %{is_end: false} -> nil
      %{value: v} -> v
    end
  end
end
```

## Test Strategy

```python
# ─── Helper ────────────────────────────────────────────────────────────────

def verify_trie(trie: Trie) -> None:
    """
    Assert trie invariants.
    1. word_count equals number of is_end nodes in the tree.
    2. Every stored word is searchable.
    3. No orphaned nodes (nodes with no children and is_end=False).
    """
    count = _count_endpoints(trie.root)
    assert count == len(trie), f"word_count {len(trie)} != actual {count}"
```

### Test Cases

```
1. Empty trie: search("any") → None, starts_with("") → False or True depending
   on semantics, len == 0, all_words == [].

2. Single insert + search: insert "hello", search("hello") → value.
   search("hell") → None. search("hellos") → None.

3. Prefix sharing: insert ["app", "apple", "apply"].
   starts_with("app") → True.
   starts_with("apz") → False.
   words_with_prefix("app") → [("app",..), ("apple",..), ("apply",..)] sorted.

4. Delete leaf: insert "apple", delete "apple".
   search("apple") → None. len == 0.
   Node cleanup: root should have no 'a' child (entire branch cleaned up).

5. Delete non-leaf: insert ["app", "apple"], delete "app".
   search("app") → None. search("apple") → still finds value.
   Node at 'p'(third level) should still exist (has child 'l').

6. Delete only shared prefix: insert ["app", "apple"], delete "app".
   words_with_prefix("app") → [("apple", ..)] only.

7. Delete all words: insert N words, delete them all.
   Verify len==0 and root.children is empty.

8. Longest prefix match:
   Insert ["a", "ab", "abc", "abcd"].
   longest_prefix_match("abcde") → ("abcd", ..)
   longest_prefix_match("xyz")   → None
   longest_prefix_match("a")     → ("a", ..)

9. All words sorted: insert in random order, all_words() returns sorted.

10. Unicode keys: insert ["café", "cafe", "caf"], search all three.

11. Empty string key: insert("", "root value"). search("") should return it.
    starts_with("") → True. words_with_prefix("") → all words including "".

12. Case sensitivity: insert "Hello", search("hello") → None (case sensitive).

13. Large scale: insert 10,000 random words, verify all searchable,
    words_with_prefix("") returns all 10,000 sorted.

14. IP routing simulation:
    Insert routes ["192", "192.168", "192.168.1", "10", "10.0"].
    longest_prefix_match("192.168.1.5") → ("192.168.1", ..)
    longest_prefix_match("192.168.2.1") → ("192.168", ..)
    longest_prefix_match("172.16.0.1") → None

15. Autocomplete simulation: insert 1000 English words.
    words_with_prefix("pre") returns all words starting with "pre" sorted.
    Verify result is identical to [w for w in words if w.startswith("pre")].
```

### Coverage Targets

- 95%+ line coverage
- Insert: new word, update existing word, empty string key
- Search: found, not found (wrong prefix), not found (prefix only)
- Delete: leaf cleanup, partial cleanup, root becomes empty
- words_with_prefix: no match, single match, multiple matches, entire dictionary
- longest_prefix_match: no match, exact match, partial match, multiple candidates

## Future Extensions

- **DT14 Radix tree** — compress single-child chains into a single edge labeled
  with the full substring. "app" → "le" is one edge, not 2 separate nodes.
  Dramatically reduces memory for tries with long uncommon tails. Redis rax.c is
  a production radix tree.
- **Ternary search trie** — each node has 3 children: less, equal, greater.
  Reduces memory vs 26-array approach while keeping O(log n + k) average lookup
  (better than map-based trie when keys are dense).
- **Bitwise trie / Patricia trie** — keys are bit strings; branching is on
  individual bits. Used for IPv4/IPv6 routing tables (32-bit or 128-bit keys).
  The internet's routing infrastructure is built on Patricia tries.
- **Persistent trie** — copy-on-write so every insert returns a new trie that
  shares unchanged structure. Enables O(1) rollback to any historical state.
  Used in functional programming and version-control systems.
- **Compressed values** — store values only at leaf nodes; intermediate nodes
  hold only routing info. Similar to the B+ tree insight: smaller internal nodes
  means better cache utilization.
