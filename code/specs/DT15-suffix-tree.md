# DT15 — Suffix Tree

## Overview

A **suffix tree** is a compressed trie (DT14) of ALL suffixes of a string S. It
encodes every possible suffix in a single tree structure so that any substring
query can be answered in O(m) time — where m is the length of the pattern —
regardless of how long S is.

Without a suffix tree, checking whether "ana" appears in "banana" requires
scanning every position: O(n · m) in the worst case. With a suffix tree, you
descend from the root once, matching characters against edge labels, and you are
done: O(m). Then collecting every occurrence takes only O(k) more time, where k
is the number of matches.

Real applications include:

- **DNA sequencing** — searching millions of base pairs for short patterns
- **Plagiarism detection** — finding the longest common substring between two documents
- **Text compression** — identifying repeated substrings (related to LZ77/LZ78)
- **`grep`-like tools** — fast multi-pattern search over large corpora

## Layer Position

```
DT13: trie
  └── DT14: radix-tree (compressed trie)
        └── DT15: suffix-tree   ← [YOU ARE HERE]
                  (radix tree of every suffix of a string)

DT16: rope            (string editing; different problem domain)
DT17: hash-functions  (hashing; completely separate family)
```

**Depends on:** DT14 (radix tree: edge compression, edge splitting).
**Extends:** DT14 by pre-populating it with every suffix of an input string.
**Used by:** bioinformatics pipelines, full-text search engines, string
algorithms courses everywhere.

## Concepts

### The Terminator Character

Before building a suffix tree, append a special character `$` that does not
appear anywhere in S. Why?

Without `$`, some suffixes are prefixes of other suffixes. For "banana":
- "ana" is a prefix of "anana"
- "a" is a prefix of "ana" and "anana"

If a suffix is a prefix of another suffix, it gets absorbed into a shared path
and loses its endpoint — the trie can no longer tell you "this suffix ends
here." The terminator forces every suffix to have a unique ending character, so
every suffix corresponds to a **leaf**, never just an internal node.

```
Without $:
  "ana" can end inside the path to "anana"
  You lose track of where the suffix ends.

With $:
  "ana$" has a unique path that no other suffix shares.
  Every suffix reaches its own leaf.
```

### From Naive Suffix Trie to Suffix Tree

**Step 1: Build the suffix trie (uncompressed).**

For S = "banana$" (length 7), the suffixes are:

```
Index 0:  banana$
Index 1:  anana$
Index 2:  nana$
Index 3:  ana$
Index 4:  na$
Index 5:  a$
Index 6:  $
```

Insert each suffix into a standard trie (DT13), one character per edge:

```
(Partial view — trie would have ~35 nodes)
root
├── $ (leaf: index 6)
├── a
│   ├── $ (leaf: index 5)
│   └── n
│       └── a
│           ├── $ (leaf: index 3)
│           └── n
│               └── a
│                   └── $ (leaf: index 1)
├── b
│   └── a
│       └── n
│           └── a
│               └── n
│                   └── a
│                       └── $ (leaf: index 0)
└── n
    └── a
        ├── $ (leaf: index 4)
        └── n
            └── a
                └── $ (leaf: index 2)
```

This trie has O(n²) nodes for a string of length n. Unacceptable for large n.

**Step 2: Compress single-child chains (radix tree / DT14).**

Collapsing every chain of single-child nodes gives the suffix tree:

```
Suffix tree for "banana$"

            root
          /   |   \
         $    b    n
(idx 6)  |    |    |
     "anana$" "anana$"  (idx 0)
     (idx 1)     \
                "a" ──────────────────
               /    \
             "$"    "na$"
           (idx 5)  (idx 4)
           "na$" from suffix "ana$"
           goes here too...

```

Let me draw this more carefully. The suffix tree for "banana$":

```
                       root
                    /   |    \
                   /    |     \
                  $    "a"    "na"      "banana$"
               (6)   /   \    /  \
                   "$"  "na" "$" "na$"
                  (5)   / \  (4)
                      "$" "na$"
                      (3)  (1 or 2)
```

A cleaner representation, labeling leaves with suffix start indices:

```
root
├── "$"                         → leaf [6]
├── "a"
│   ├── "$"                     → leaf [5]
│   └── "na"
│       ├── "$"                 → leaf [3]
│       └── "na$"               → leaf [1]
├── "banana$"                   → leaf [0]
└── "na"
    ├── "$"                     → leaf [4]
    └── "na$"                   → leaf [2]
```

Every path from root to a leaf spells out a complete suffix. Every internal
node represents a substring that occurs **more than once** in S (it is a
shared prefix of at least two suffixes). This is why the **deepest internal
node** gives the longest repeated substring.

### Suffix Links

Every internal node u (except the root) has a **suffix link** — a pointer to
another internal node v such that:

```
if path(root → u) spells "xα"
then path(root → v) spells "α"

In other words: strip the first character.
```

Example for "banana$":

```
node spelling "ana"  →  suffix link →  node spelling "na"
node spelling "na"   →  suffix link →  node spelling "a"
node spelling "a"    →  suffix link →  root
```

Suffix links allow Ukkonen's algorithm to avoid rescanning from the root when
extending suffixes. Without suffix links, you would redo prefix-scanning work
for every new suffix. With them, you jump directly to the right location.

### Ukkonen's Algorithm (O(n) Construction)

Ukkonen's algorithm builds the suffix tree in O(n) time using three key ideas:

**Idea 1 — Implicit extension.**
When you extend the tree for character S[i], you do not have to do anything for
suffixes that already end at a leaf. The implicit rule: all active leaves grow
by one character automatically (conceptually; in practice you defer this with
a global end pointer).

**Idea 2 — Three extension rules.**
For each suffix to be extended with character c:
- **Rule 1**: suffix ends at a leaf. Extend the leaf's edge label by c. (Free — done via global end pointer.)
- **Rule 2**: suffix ends at an internal node or mid-edge and the next character is NOT c. Create a new leaf (and possibly split the edge and create a new internal node). This is the only rule that actually allocates nodes.
- **Rule 3**: suffix ends at an internal node or mid-edge and the next character IS already c. Do nothing — the suffix is implicitly present. **Stop processing further suffixes for this character.**

**Idea 3 — Active point.**
Rather than rescanning from the root for every suffix, maintain three variables:
```
active_node   — where in the tree we currently are
active_edge   — which edge we are on (first character)
active_length — how far along that edge we are
```
Together they encode the exact point in the tree where the next extension needs
to happen. Suffix links teleport `active_node` when we apply Rule 2 and
create a new internal node.

The net effect: each of the n characters triggers at most O(1) amortized work,
giving O(n) total construction time.

> For a full pedagogical walkthrough of Ukkonen's, see Tushar Roy's series
> or Giegerich & Kurtz "From Ukkonen to McCreight and Weiner: A Unifying View."

### Longest Repeated Substring

Every internal node represents a substring that occurs at least twice (it has
at least two descendant leaves). The **deepest** internal node — meaning the
one with the longest path label from the root — gives the longest repeated
substring.

```
"banana$" — internal nodes and their path labels:
  "a"    (depth 1)
  "na"   (depth 2)
  "ana"  (depth 3)   ← deepest internal node

Longest repeated substring: "ana"
(appears at indices 1 and 3)
```

### Generalized Suffix Tree (Longest Common Substring)

To find the longest common substring between strings S1 and S2, build a
**generalized suffix tree**: concatenate S1 + "#" + S2 + "$" (using two
distinct terminators) and build the suffix tree of the combined string. Then
find the deepest internal node that has descendant leaves from BOTH S1 and S2.

## Representation

### Node

```
SuffixTreeNode:
  children:      dict[char, SuffixTreeNode]   # keyed by first char of edge
  edge_start:    int                          # index into original string
  edge_end:      int | None                  # None means "open" (leaf grows)
  suffix_index:  int | None                  # set only for leaves
  suffix_link:   SuffixTreeNode | None       # for Ukkonen's algorithm
```

The edge label for a node is `S[edge_start : edge_end]`. Leaves use a shared
`end` pointer (an integer that grows as we process characters), so all leaf
edges grow in O(1) without per-character allocation.

### Tree

```
SuffixTree:
  root:   SuffixTreeNode
  text:   str                  # S with terminator appended
  size:   int                  # len(text)
```

Nodes are connected by references, not arrays. For large alphabets (e.g., DNA
with 4 symbols), children can be a fixed-size array of 4; for Unicode text, a
hash map is appropriate.

## Algorithms (Pure Functions)

### `build(s: str) → SuffixTree`

Naive O(n²) approach (insert each suffix as a path):

```
1. Append "$" to s → text
2. Create root node
3. For i in 0..len(text):
     suffix = text[i:]
     current = root
     j = 0
     while j < len(suffix):
       c = suffix[j]
       if c not in current.children:
         # No match: add a leaf with the rest of the suffix
         leaf = new_node(edge_start=i+j, suffix_index=i)
         current.children[c] = leaf
         break
       else:
         child, edge = current.children[c]
         # Walk along the edge
         k = 0
         while k < len(edge) and j < len(suffix) and suffix[j] == edge[k]:
           j += 1
           k += 1
         if k == len(edge):
           # Consumed the entire edge: move to child
           current = child
         else:
           # Mid-edge mismatch: split the edge
           split = new_internal_node(edge_start = child.edge_start,
                                     edge_end   = child.edge_start + k)
           current.children[c] = split
           child.edge_start += k
           split.children[edge[k]] = child
           leaf = new_node(edge_start=i+j, suffix_index=i)
           split.children[suffix[j]] = leaf
           break
4. Return SuffixTree(root, text)
```

Note: `build_ukkonen(s)` implements the O(n) version using suffix links and the
active-point mechanism described above. In production, always use Ukkonen's.

### `search(tree, pattern) → list[int]`

Returns all starting indices where `pattern` occurs in the original string.

```
1. Start at root, position = 0 in pattern
2. Walk the tree matching pattern characters against edge labels
3. If a mismatch: return []  (pattern not found)
4. If pattern exhausted before or at a node: collect all leaf indices
   in the subtree rooted here (DFS) — each leaf index is an occurrence
5. Return sorted list of indices
Time: O(m + k) where m = len(pattern), k = number of occurrences
```

### `count_occurrences(tree, pattern) → int`

Same as search but count leaves instead of collecting them.
```
Time: O(m + k)
```

### `longest_repeated_substring(tree) → str`

```
1. DFS over all internal nodes
2. Track the node with the maximum path-label depth
3. Reconstruct the path label by tracing from root to that node
4. Return the substring
Time: O(n)
```

### `longest_common_substring(s1, s2) → str`

```
1. text = s1 + "#" + s2 + "$"
2. tree = build(text)
3. Tag each leaf as belonging to s1 (index < len(s1)) or s2
4. For each internal node, propagate tags upward (DFS)
5. Find deepest internal node whose subtree has leaves from BOTH s1 and s2
6. Return its path label
Time: O(n1 + n2)
```

## Public API

```python
class SuffixTreeNode:
    children:     dict[str, "SuffixTreeNode"]
    edge_start:   int
    edge_end:     int | None          # None = open leaf
    suffix_index: int | None          # set for leaves only
    suffix_link:  "SuffixTreeNode | None"

class SuffixTree:
    root: SuffixTreeNode
    text: str                         # original string + terminator

# Construction
def build(s: str) -> SuffixTree: ...                  # O(n^2) naive
def build_ukkonen(s: str) -> SuffixTree: ...          # O(n) Ukkonen

# Querying
def search(tree: SuffixTree, pattern: str) -> list[int]: ...
def count_occurrences(tree: SuffixTree, pattern: str) -> int: ...
def longest_repeated_substring(tree: SuffixTree) -> str: ...
def longest_common_substring(s1: str, s2: str) -> str: ...

# Introspection
def all_suffixes(tree: SuffixTree) -> list[str]: ...  # DFS leaf collection
def node_count(tree: SuffixTree) -> int: ...
```

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

```python
# Python
class SuffixTreeNode(RadixTreeNode):
    suffix_index: int | None = None
    suffix_link:  "SuffixTreeNode | None" = None

class SuffixTree(RadixTree):
    text: str
    def build(self, s: str) -> None: ...
    def search(self, pattern: str) -> list[int]: ...
```

```typescript
// TypeScript
class SuffixTreeNode extends RadixTreeNode {
  suffixIndex: number | null;
  suffixLink:  SuffixTreeNode | null;
}

class SuffixTree extends RadixTree {
  text: string;
  search(pattern: string): number[] { ... }
}
```

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — composition, not inheritance
pub struct SuffixTreeNode {
    pub children:     HashMap<char, Box<SuffixTreeNode>>,
    pub edge_start:   usize,
    pub edge_end:     Option<usize>,   // None = open leaf
    pub suffix_index: Option<usize>,
    pub suffix_link:  Option<*mut SuffixTreeNode>,  // raw ptr for Ukkonen
}

pub struct SuffixTree {
    pub root: Box<SuffixTreeNode>,
    pub text: Vec<char>,
}

// Pure functions as free functions or impl blocks
pub fn build(s: &str) -> SuffixTree { ... }
pub fn search(tree: &SuffixTree, pattern: &str) -> Vec<usize> { ... }
```

```go
// Go — struct embedding for shared node fields
type SuffixTreeNode struct {
    Children    map[byte]*SuffixTreeNode
    EdgeStart   int
    EdgeEnd     *int          // nil = open leaf
    SuffixIndex *int          // nil for internal nodes
    SuffixLink  *SuffixTreeNode
}

type SuffixTree struct {
    Root *SuffixTreeNode
    Text []byte
}

func Build(s string) *SuffixTree { ... }
func Search(tree *SuffixTree, pattern string) []int { ... }
```

```elixir
# Elixir — tagged maps
defmodule SuffixTree do
  defstruct [:root, :text]
  # Nodes are plain maps: %{children: %{}, edge_start: n, edge_end: n, suffix_index: n}
  def build(s), do: ...
  def search(tree, pattern), do: ...
end
```

## Test Strategy

### Unit tests

```
build("$")                   → tree with single leaf, one edge "$"
build("a$")                  → root with one child, leaf index 0
build("aa$")                 → tree with two leaves
build("banana$")             → 7 leaves (one per suffix); spot-check structure

search(tree("banana$"), "ana")    → [1, 3]
search(tree("banana$"), "nan")    → [2]
search(tree("banana$"), "xyz")    → []
search(tree("banana$"), "")       → all indices (convention: all positions)
search(tree("banana$"), "banana") → [0]

count_occurrences(tree("banana$"), "a")   → 3
count_occurrences(tree("banana$"), "na")  → 2
count_occurrences(tree("banana$"), "xyz") → 0

longest_repeated_substring(tree("banana$"))     → "ana"
longest_repeated_substring(tree("abcabc$"))     → "abc"
longest_repeated_substring(tree("aaa$"))        → "aa"
longest_repeated_substring(tree("abcdef$"))     → "" (no repeats)

longest_common_substring("abcdef", "bcde")  → "bcde"
longest_common_substring("ABAB", "BABA")    → "BAB" or "ABA" (length 3)
longest_common_substring("abc", "xyz")      → ""
```

### Property-based tests

- For any string S, the number of leaves in `build(S + "$")` equals `len(S) + 1`.
- Every suffix of S is found by `search(tree, suffix)`.
- `search(tree, pattern)` results match naive scan.
- `all_suffixes(tree)` == sorted list of all suffixes of S.

### Performance tests

- `build_ukkonen` on a 100,000-character string completes in < 1 second.
- `search` on a 100,000-character tree returns results in < 1 ms for pattern length 10.

## Future Extensions

- **Suffix arrays**: A flat-array encoding of the suffix tree. Requires O(n)
  memory (vs O(n) but with larger constants for suffix trees). Often preferred
  in practice. DT15 provides the conceptual foundation.
- **LCP arrays**: Alongside suffix arrays, encode longest common prefix between
  adjacent suffixes — enables many O(n) string algorithms.
- **Aho-Corasick**: Extends tries with failure links; efficient multi-pattern
  search. Conceptually related to suffix links.
- **Compressed suffix arrays**: Reduce memory to O(n log n) bits using
  Burrows-Wheeler transform — the core of `bzip2` and `BWA` (bioinformatics).
