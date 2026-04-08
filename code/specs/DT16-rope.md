# DT16 — Rope

## Overview

A **rope** is a binary tree (DT03) where the **leaves** store fixed string
chunks and each **internal node** stores a single integer — the **weight** —
equal to the total character count of its entire left subtree.

Ropes solve a fundamental problem in text editors: ordinary strings are
terrible at mid-document edits. If you store a 10 MB document as a single
`str` and the user inserts one character at position 50,000, the runtime must
shift every character from position 50,001 onward — that is potentially
millions of memory operations for a single keystroke. Ropes bring insert and
delete down to O(log n) by splitting the edit into tree pointer operations
rather than memory moves.

Real editors that use ropes or close relatives:
- **VS Code** — uses a "piece tree" (a rope variant)
- **xi-editor** (now archived, but influential) — pure rope
- **GNU Emacs** — gap buffer (a different O(log n) technique; ropes are an
  alternative)
- **Sublime Text** — undisclosed, but benchmarks match rope characteristics

## Layer Position

```
DT02: tree  (general tree)
  └── DT03: binary-tree  (two children per node)
        └── DT16: rope   ← [YOU ARE HERE]
              (binary tree specialized for string concatenation and slicing)

DT04: heap          (binary tree; priority ordering; unrelated problem)
DT15: suffix-tree   (trie-based; string search; unrelated problem)
```

**Depends on:** DT03 (binary tree: left/right children, DFS traversal).
**Specializes:** DT03 by giving internal nodes a "weight" field and leaves a
"chunk" field.
**Used by:** text editors, diff tools, collaborative editing engines (OT/CRDT).

## Concepts

### Why Flat Strings Are Bad for Text Editors

Consider a document as a plain `str` of length n:

```
"Hello, world! This is a long document......"
  0     6      14
```

**Inserting** "NEW " at position 7:
```
Before: H e l l o ,   w o r l d !   T h i s ...
After:  H e l l o ,   N E W   w o r l d !   T h i s ...
```

Every character from index 7 onward must be copied one position to the right.
For a 1 MB document, that is ~1,000,000 copy operations per keystroke.

| Operation        | Array/str | Rope      |
|------------------|-----------|-----------|
| Index (i-th char)| O(1)      | O(log n)  |
| Insert at i      | O(n)      | O(log n)  |
| Delete at i      | O(n)      | O(log n)  |
| Concatenate      | O(n)      | O(1)*     |
| Split at i       | O(n)      | O(log n)  |
| Iterate          | O(n)      | O(n)      |

*O(1) for concat if we allow unbalanced trees; O(log n) if we rebalance.

The O(1) concatenation is the most dramatic gain. Creating a new rope from two
existing ropes requires allocating exactly one new internal node — no data is
copied. This is the core trick.

### The Weight Field

Every **internal node** stores a single integer called the **weight**:

```
weight = total number of characters in the LEFT subtree
```

Note carefully: the weight counts only the left subtree, not the whole
subtree. This asymmetry is intentional — it lets you navigate to any character
in O(log n) using the algorithm described below.

```
Example: Rope for "Hello, world!"
(split arbitrarily into chunks "Hello, " and "world!")

         (weight=7)
          /       \
      "Hello, "  "world!"
      (leaf)     (leaf)

The root weight is 7 because "Hello, " has 7 characters.
```

A more complex rope for "Hello, world! Goodbye!":

```
                  (weight=13)
                 /           \
          (weight=7)        "Goodbye!"
          /       \          (leaf)
       "Hello, " "world!"
       (leaf)    (leaf)

Root weight = 13 = len("Hello, ") + len("world!") = 7 + 6
```

### Index Access (O(log n))

To find the character at position i, start at the root and descend:

```
index(node, i):
  if node is a leaf:
    return node.chunk[i]
  if i < node.weight:
    return index(node.left, i)         # character is in the left subtree
  else:
    return index(node.right, i - node.weight)  # subtract left size, go right
```

Trace: find character at index 9 in "Hello, world!":

```
root (weight=7): 9 >= 7, go right with i = 9 - 7 = 2
"world!" (leaf): return "world!"[2] = 'r'
```

Each step moves down one level, so depth = O(log n) for a balanced tree.

### Concatenation (O(1))

Concatenating rope A and rope B:

```
concat(A, B):
  new_root = InternalNode(
    left   = A,
    right  = B,
    weight = total_length(A)
  )
  return new_root
```

No data is copied. "Hello, " and "world!" remain where they are; only a new
root node is created. For a document split into 1,000 chunks, concatenating
two large ropes is still one allocation.

```
Before:
  rope_a: "Hello, "     rope_b: "world!"

After concat(rope_a, rope_b):
       (weight=7)
      /          \
  "Hello, "    "world!"
```

### Split at Position i (O(log n))

Splitting creates two ropes: the left contains characters [0, i) and the right
contains [i, n).

The split algorithm descends the tree using the weight field (same navigation
as index) and restructures edges as it backtracks:

```
split(node, i):
  if node is a leaf:
    return (leaf(chunk[:i]), leaf(chunk[i:]))

  if i < node.weight:
    (left_part, right_part) = split(node.left, i)
    return (left_part,
            InternalNode(right_part, node.right,
                         weight=total_length(right_part)))
  else:
    (left_part, right_part) = split(node.right, i - node.weight)
    return (InternalNode(node.left, left_part,
                         weight=node.weight),
            right_part)
```

Trace: split "Hello, world!" at position 7:

```
root (weight=7): 7 == weight, go right with i = 7 - 7 = 0
"world!" (leaf): split at 0 → ("", "world!")
backtrack: return (InternalNode("Hello, ", ""), "world!")
           = ("Hello, ", "world!")
```

### Insertion (O(log n))

Insert string s at position i:

```
insert(rope, i, s):
  (left, right) = split(rope, i)
  return concat(concat(left, rope(s)), right)
```

Three operations: one split, two concats. Each is O(log n), so insertion is
O(log n). No bulk data movement.

### Deletion (O(log n))

Delete length characters starting at position start:

```
delete(rope, start, length):
  (left, rest)  = split(rope, start)
  (mid, right)  = split(rest, length)
  # mid is discarded
  return concat(left, right)
```

Two splits, one concat. O(log n).

### Rebalancing

After many concatenations and splits, the rope can become a heavily left- or
right-skewed tree with O(n) height, degrading all operations back to O(n).

Two rebalancing strategies:

**Strategy 1 — Fibonacci bounds (Boehm et al. 1995)**

Define Fibonacci(k) as the kth Fibonacci number. A rope is balanced if, for
every subtree of depth d, the length of that subtree is at least Fibonacci(d+2).

This mirrors the AVL condition (DT08): a balanced rope cannot be shorter than
a certain minimum length relative to its depth. The minimum depth of a balanced
rope of length n is O(log_φ n) where φ = 1.618 (golden ratio).

Rebalancing algorithm:
1. Collect all leaves in left-to-right order.
2. Divide-and-conquer: repeatedly split the leaf list in half and build
   internal nodes bottom-up.
3. Result: a balanced tree with O(log n) depth.

**Strategy 2 — Trigger rebalance lazily**

After each operation, check `is_balanced(rope)`. If not, rebuild from leaves.
In practice, rebalance only when depth exceeds 2 × log2(length).

### is_balanced Check

```
is_balanced(node) → bool:
  d = depth(node)
  n = length(node)
  return n >= fibonacci(d + 2)
```

## Representation

### Node (discriminated union / algebraic type)

```
LeafNode:
  chunk: str                  # the actual characters

InternalNode:
  weight: int                 # length of left subtree
  left:   Node                # left subtree
  right:  Node                # right subtree
```

Leaf nodes store data. Internal nodes store only a weight and two pointers.
There is no "value" at an internal node — all characters live in leaves.

### Rope

```
Rope:
  root: Node | None            # None represents the empty rope
```

Length of the whole rope = `weight(root) + length(root.right)`. Or maintain a
cached `total_length` field at the root for O(1) length queries.

## Algorithms (Pure Functions)

### `length(rope) → int`

```
length(None)      → 0
length(leaf)      → len(leaf.chunk)
length(internal)  → internal.weight + length(internal.right)
```

If total_length is cached at the root, this is O(1).

### `index(rope, i) → char`

```
index(None, i)    → error: out of bounds
index(leaf, i)    → leaf.chunk[i]
index(internal, i):
  if i < internal.weight: return index(internal.left, i)
  else:                    return index(internal.right, i - internal.weight)
```

### `to_string(rope) → str`

```
to_string(None)      → ""
to_string(leaf)      → leaf.chunk
to_string(internal)  → to_string(internal.left) + to_string(internal.right)
```

O(n) — visits every leaf.

### `concat(rope1, rope2) → Rope`

```
concat(None, r)    → r
concat(r, None)    → r
concat(r1, r2):
  return InternalNode(weight=length(r1), left=r1, right=r2)
```

O(1).

### `split(rope, i) → (Rope, Rope)`

See Concepts section above for full algorithm. O(log n).

### `insert(rope, i, s) → Rope`

```
(left, right) = split(rope, i)
return concat(concat(left, rope_from_string(s)), right)
```

O(log n).

### `delete(rope, start, length) → Rope`

```
(left, rest)  = split(rope, start)
(_, right)    = split(rest, length)
return concat(left, right)
```

O(log n).

### `is_balanced(rope) → bool`

```
is_balanced(None)     → True
is_balanced(leaf)     → True
is_balanced(internal):
  d = depth(internal)
  n = length(internal)
  return n >= fibonacci(d + 2)
         and is_balanced(internal.left)
         and is_balanced(internal.right)
```

### `rebalance(rope) → Rope`

```
leaves = collect_leaves(rope)   # DFS, left-to-right
return build_balanced(leaves)   # divide-and-conquer

build_balanced(leaves):
  if len(leaves) == 1: return leaves[0]
  mid = len(leaves) // 2
  left  = build_balanced(leaves[:mid])
  right = build_balanced(leaves[mid:])
  return InternalNode(weight=length(left), left=left, right=right)
```

O(n).

## Public API

```python
from dataclasses import dataclass
from typing import Union

@dataclass
class LeafNode:
    chunk: str

@dataclass
class InternalNode:
    weight: int
    left:   "Node"
    right:  "Node"

Node = Union[LeafNode, InternalNode, None]

@dataclass
class Rope:
    root: Node = None

# Construction
def rope_from_string(s: str) -> Rope: ...      # O(1): single leaf
def rope_empty() -> Rope: ...                  # Rope(root=None)

# Core operations
def length(rope: Rope) -> int: ...             # O(1) if cached
def index(rope: Rope, i: int) -> str: ...      # O(log n)
def to_string(rope: Rope) -> str: ...          # O(n)

# Structural operations (all return new Rope — functional/immutable style)
def concat(r1: Rope, r2: Rope) -> Rope: ...    # O(1)
def split(rope: Rope, i: int) -> tuple[Rope, Rope]: ...  # O(log n)
def insert(rope: Rope, i: int, s: str) -> Rope: ...      # O(log n)
def delete(rope: Rope, start: int, length: int) -> Rope: ... # O(log n)
def substring(rope: Rope, start: int, end: int) -> str: ... # O(log n + k)

# Balance
def depth(rope: Rope) -> int: ...              # O(n)
def is_balanced(rope: Rope) -> bool: ...       # O(n)
def rebalance(rope: Rope) -> Rope: ...         # O(n)
```

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

```python
# Python — use a sealed/discriminated base class
from abc import ABC, abstractmethod

class RopeNode(ABC):
    @abstractmethod
    def length(self) -> int: ...

class RopeLeaf(RopeNode):
    def __init__(self, chunk: str): self.chunk = chunk
    def length(self) -> int: return len(self.chunk)

class RopeInternal(RopeNode):
    def __init__(self, weight: int, left: RopeNode, right: RopeNode):
        self.weight = weight
        self.left   = left
        self.right  = right
    def length(self) -> int: return self.weight + self.right.length()
```

```typescript
// TypeScript — discriminated union
type RopeNode =
  | { kind: "leaf";     chunk: string }
  | { kind: "internal"; weight: number; left: RopeNode; right: RopeNode };

function ropeLength(node: RopeNode): number {
  return node.kind === "leaf"
    ? node.chunk.length
    : node.weight + ropeLength(node.right);
}
```

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — enum for the discriminated union
#[derive(Debug, Clone)]
pub enum RopeNode {
    Leaf(String),
    Internal {
        weight: usize,
        left:   Box<RopeNode>,
        right:  Box<RopeNode>,
    },
}

pub struct Rope {
    pub root: Option<Box<RopeNode>>,
}

pub fn concat(r1: Rope, r2: Rope) -> Rope { ... }
pub fn split(rope: Rope, i: usize) -> (Rope, Rope) { ... }
pub fn rope_index(rope: &Rope, i: usize) -> Option<char> { ... }
```

```go
// Go — interface-based discrimination
type RopeNode interface {
    ropeLength() int
}

type RopeLeaf struct {
    Chunk string
}

type RopeInternal struct {
    Weight int
    Left   RopeNode
    Right  RopeNode
}

type Rope struct {
    Root RopeNode
}

func Concat(r1, r2 Rope) Rope { ... }
func Split(r Rope, i int) (Rope, Rope) { ... }
```

```elixir
# Elixir — tagged tuples
defmodule Rope do
  # Nodes: {:leaf, chunk} | {:internal, weight, left, right}

  def concat(nil, r), do: r
  def concat(r, nil), do: r
  def concat(r1, r2), do: {:internal, length(r1), r1, r2}

  def length(nil), do: 0
  def length({:leaf, chunk}), do: String.length(chunk)
  def length({:internal, w, _, right}), do: w + length(right)
end
```

## Test Strategy

### Unit tests

```
# Construction
rope_from_string("")          → empty rope, length 0
rope_from_string("hello")     → leaf node, length 5
to_string(rope_from_string("hello")) → "hello"

# Index
index(rope_from_string("hello"), 0)  → 'h'
index(rope_from_string("hello"), 4)  → 'o'
index(rope_from_string("hello"), 5)  → error (out of bounds)

# Concat
r = concat(rope_from_string("Hello, "), rope_from_string("world!"))
to_string(r)     → "Hello, world!"
length(r)        → 13
index(r, 7)      → 'w'
index(r, 12)     → '!'

# Split
(l, r) = split(rope_from_string("Hello, world!"), 7)
to_string(l) → "Hello, "
to_string(r) → "world!"

# Insert
r = insert(rope_from_string("Hello!"), 5, ",")
to_string(r) → "Hello,!"

r = insert(rope_from_string("Hello!"), 0, "Say: ")
to_string(r) → "Say: Hello!"

# Delete
r = delete(rope_from_string("Hello, world!"), 5, 2)
to_string(r) → "Helloworld!"

# is_balanced and rebalance
# Build a degenerate rope by 100 right-concatenations
r = fold 100 single-char ropes with concat(acc, next)
is_balanced(r) → False  (depth 100, length 100: fib(102) >> 100)
r2 = rebalance(r)
is_balanced(r2) → True
to_string(r2) == to_string(r)  → True  (content unchanged)
```

### Property-based tests

- For all ropes r and positions 0 ≤ i ≤ length(r):
  `to_string(concat(l, r)) == to_string(l) + to_string(r)`
- `to_string(insert(r, i, s)) == to_string(r)[:i] + s + to_string(r)[i:]`
- `to_string(delete(r, start, n)) == to_string(r)[:start] + to_string(r)[start+n:]`
- `concat(split(r, i)) == r` (up to structural equality, content equal)
- `rebalance(r)` produces `is_balanced == True` and same string content.

### Performance tests

- 10,000 sequential inserts at random positions into a 100,000-character rope
  complete in < 1 second.
- `index` on a balanced rope of depth 20 (2^20 ≈ 1M chars) executes in < 1 μs.

## Future Extensions

- **Gap buffer**: an alternative O(1) insert at cursor (but O(n) for
  arbitrary-position inserts). Simpler to implement; worth comparing.
- **Piece table**: used by VS Code. Like a rope, but tracks which segments come
  from the original file vs. an "add" buffer. Avoids string allocation.
- **CRDT ropes**: used in collaborative editors (e.g., Yjs) where multiple
  users edit concurrently. Each character gets a unique logical timestamp.
- **Persistent ropes**: since concat and split are mostly non-destructive (they
  create new nodes), ropes naturally support persistent/immutable semantics
  with structural sharing.
