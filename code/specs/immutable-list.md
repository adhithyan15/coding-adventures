# ImmutableList

## Overview

An immutable list is a persistent data structure — once created, it never changes.
Every "modification" (push, set, pop) returns a **new** list, leaving the original
intact. This sounds wasteful, but the trick is **structural sharing**: the new list
reuses most of the old list's memory, only allocating new nodes along the path that
changed.

### Why Does This Matter?

**Safety.** When data structures can't be mutated, entire categories of bugs
disappear: no iterator invalidation, no data races between threads, no spooky
action-at-a-distance where one function modifies a list another function is
reading. You can pass an immutable list to any function and know it won't be
changed out from under you.

**Concurrency.** Because immutable data is inherently thread-safe, there's no
need for locks, mutexes, or atomic operations when reading. Multiple threads can
share the same list simultaneously with zero synchronization overhead. Only the
writer pays the cost of creating a new version.

**Time travel.** Every version of the list persists. You can keep a reference to
"the list before the last 50 pushes" and it's still valid, still O(1) to access.
This enables undo/redo, transactional semantics, and snapshotting for free.

### The Design: 32-Way Trie with Tail Buffer

Clojure's `PersistentVector` (designed by Rich Hickey, based on Phil Bagwell's
work on Hash Array Mapped Tries) achieves near-constant-time operations on an
immutable list by combining two ideas:

1. **A wide trie (32-way branching tree).** Instead of a binary tree where each
   node has 2 children, every node has up to 32 children. This means a tree
   holding 1 million elements is only 4 levels deep (32^4 = 1,048,576). Index
   lookup walks at most 4 levels — effectively O(1) for any practical size.

2. **A tail buffer.** The last 32 elements live in a flat array outside the trie.
   Most `push` operations just append to this buffer — no tree traversal needed.
   Only when the tail fills up (every 32nd push) does the buffer get promoted into
   the trie as a new leaf node. This means ~97% of pushes are a simple array
   append.

### Cache Locality

Why 32? Because 32 elements per node means each node fits in 1-2 CPU cache lines
(a cache line is typically 64 bytes). When the CPU loads a node, all 32 children
or elements come with it. Compare this to a binary tree where each node holds just
2 pointers and every access is a cache miss. The 32-way branching factor is
specifically chosen to maximize cache utilization while keeping tree depth minimal.

```
    Depth vs. capacity for a 32-way trie:

    Depth 1:  32^1 =            32 elements
    Depth 2:  32^2 =         1,024 elements
    Depth 3:  32^3 =        32,768 elements
    Depth 4:  32^4 =     1,048,576 elements
    Depth 5:  32^5 =    33,554,432 elements
    Depth 6:  32^6 = 1,073,741,824 elements

    A depth-4 trie holds over a million elements. For most applications,
    you'll never exceed depth 5 or 6. This is why O(log32 n) is
    effectively O(1) — the "log" never exceeds single digits.
```

## Layer Position

ImmutableList is a foundation package for the immutable collections family. It has
no dependencies on other packages in this project. Future immutable collections
(ImmutableMap, ImmutableSet) will be separate packages at the same layer, sharing
the same design philosophy of structural sharing via wide tries.

```
                      Future Consumers
    ┌───────────────┬───────────────┬────────────────────┐
    │ Undo/Redo     │ Transactional │ Concurrent         │
    │ History       │ State         │ Snapshots          │
    │ (list of      │ (immutable    │ (lock-free shared  │
    │  versions)    │  state tree)  │  data)             │
    └──────┬────────┴───────┬───────┴──────────┬─────────┘
           │                │                  │
           ▼                ▼                  ▼
    ┌──────────────────────────────────────────────────────┐
    │               Immutable Collections                   │
    │                                                       │
    │  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
    │  │ImmutableList │  │ImmutableMap  │  │ImmutableSet │ │
    │  │(this spec)   │  │(future:HAMT) │  │(future)     │ │
    │  └──────────────┘  └──────────────┘  └─────────────┘ │
    │                                                       │
    │  Foundation layer. No dependencies.                    │
    │  Structural sharing via Arc + 32-way tries.           │
    └──────────────────────────────────────────────────────┘
```

**Depends on:** Nothing (standalone foundation).
**Used by:** Future `undo-history`, `transactional-state`, and any package needing
persistent (in the functional programming sense) ordered collections.

## Package Matrix

ImmutableList ships as 5 native-only packages. Structural sharing requires
reference-counted pointers (`Arc` in Rust) to share tree nodes between versions,
which is fundamentally a systems-level concept. There are no pure implementations —
the Rust core is the single source of truth, exposed to other languages via FFI
bridges.

| #  | Package                     | Language       | Description                              |
|----|-----------------------------|----------------|------------------------------------------|
| 1  | `immutable-list`            | Rust           | Core implementation, canonical reference |
| 2  | `immutable-list-native`     | Python (C ext) | Python bridge via `python-bridge`        |
| 3  | `immutable_list_native`     | Ruby (C ext)   | Ruby bridge via `ruby-bridge`            |
| 4  | `immutable-list-native`     | TypeScript     | Node.js bridge via `node-bridge`         |
| 5  | `immutable-list`            | WASM           | Browser bridge via `wasm-bindgen`        |

Why no pure implementations? A Python or Ruby version would need to simulate
`Arc`-style reference counting manually. The resulting code would be slow (no
cache locality, GC pressure from millions of small objects) and wouldn't teach
the underlying algorithm any better than the Rust source with good comments.
The Rust implementation *is* the pedagogical artifact.

## Concepts

### The 32-Way Trie Structure

The trie is a tree where every internal node has exactly 32 child slots, and every
leaf node has exactly 32 element slots. Elements are stored only in leaves. Internal
nodes exist only to route index lookups to the correct leaf.

```
    A list with 96 elements (3 full leaves):

                        ┌─────────────────────┐
                        │    Internal Node     │
                        │                      │
                        │  [0] [1] [2] [3]...  │   ◄── 32 child slots
                        │   │   │   │   (nil)  │       (29 are nil)
                        └───┼───┼───┼──────────┘
                            │   │   │
                   ┌────────┘   │   └────────┐
                   ▼            ▼            ▼
              ┌─────────┐ ┌─────────┐ ┌─────────┐
              │ Leaf 0   │ │ Leaf 1   │ │ Leaf 2   │
              │          │ │          │ │          │
              │ elem 0   │ │ elem 32  │ │ elem 64  │
              │ elem 1   │ │ elem 33  │ │ elem 65  │
              │ ...      │ │ ...      │ │ ...      │
              │ elem 31  │ │ elem 63  │ │ elem 95  │
              └──────────┘ └──────────┘ └──────────┘

    Each leaf holds 32 consecutive elements.
    The internal node's child[i] points to the leaf holding elements
    i*32 through i*32+31.
```

For larger lists, the trie grows deeper:

```
    A list with 2000 elements (depth 2):

                           ┌───────────────┐
                           │   Root (L2)    │
                           │  [0] [1] ...   │   ◄── up to 32 children
                           └──┼───┼─────────┘
                              │   │
                  ┌───────────┘   └───────────┐
                  ▼                            ▼
         ┌────────────────┐           ┌────────────────┐
         │ Internal (L1)  │           │ Internal (L1)  │
         │ [0] [1]...[31] │           │ [0] [1]...[30] │
         └──┼───┼─────┼───┘           └──┼───┼─────┼───┘
            │   │     │                  │   │     │
            ▼   ▼     ▼                  ▼   ▼     ▼
          ┌───┐┌───┐┌───┐             ┌───┐┌───┐┌───┐
          │L  ││L  ││L  │  ...        │L  ││L  ││L  │
          │e0 ││e32││e.. │             │e  ││e  ││e  │
          │-31││-63││    │             │   ││   ││   │
          └───┘└───┘└───┘             └───┘└───┘└───┘

    First internal node covers elements 0-1023 (32 leaves x 32 elements).
    Second internal node covers elements 1024-1999.
    Total capacity at depth 2: 32 x 32 = 1024 elements per subtree,
    32 subtrees = 32,768 max elements.
```

### Tail Buffer Optimization

The tail buffer is the key to making `push` fast. The last block of elements
(up to 32) lives outside the trie in a separate flat array. This avoids tree
traversal for the most common operation.

```
    A list with 35 elements:

    Trie (holds elements 0-31):            Tail buffer (holds elements 32-34):

    ┌──────────────┐                       ┌────────────────────────┐
    │  Leaf Node   │                       │  "elem32"              │
    │              │                       │  "elem33"              │
    │  elem 0     │                       │  "elem34"              │
    │  elem 1     │                       │  (29 empty slots)      │
    │  ...         │                       └────────────────────────┘
    │  elem 31    │
    └──────────────┘                       ▲
                                           │
                                    Most pushes just append here.
                                    No tree nodes created.
                                    No Arc cloning.
                                    Just a simple array write.
```

When the tail fills up (reaches 32 elements), it gets promoted into the trie as
a new leaf node, and a fresh empty tail is created:

```
    Before promotion (tail is full, 64 elements total):

    Trie: [Leaf 0: elem 0-31]     Tail: [elem 32-63] (FULL!)

    After promotion (push of element 64):

    Trie:                          Tail:
    ┌─────────────┐                ┌──────────────┐
    │  Internal   │                │  "elem64"    │
    │  [0]  [1]   │                │  (31 empty)  │
    │   │    │    │                └──────────────┘
    └───┼────┼────┘
        │    │
        ▼    ▼
    ┌──────┐┌──────┐
    │Leaf 0││Leaf 1│  ◄── Old tail became Leaf 1
    │ 0-31 ││32-63 │
    └──────┘└──────┘
```

### Bit Partitioning for Index Lookup

To find element at index `i`, we use bit partitioning. The index is split into
5-bit chunks (because 2^5 = 32), each chunk selecting a child at one level of
the trie.

```
    The formula at each level: child_index = (i >> shift) & 0x1F

    where shift = depth * 5, and 0x1F = 31 = 0b11111 (5-bit mask)

    Example: get(index = 1000) in a depth-2 trie (shift starts at 10)

    index = 1000
    binary: 0b00000_01111_01000

    Level 0 (root, shift=10):  (1000 >> 10) & 0x1F = 0  & 31 = 0
    Level 1 (shift=5):         (1000 >>  5) & 0x1F = 31 & 31 = 31
    Level 2 (leaf, shift=0):   (1000 >>  0) & 0x1F = 8  & 31 = 8

    Path: root.children[0].children[31].elements[8]

    Step by step:
    ┌───────────────┐
    │  Root          │
    │  child[0] ─────┼──►  ┌───────────────┐
    │                │     │  Internal      │
    └───────────────┘     │  child[31] ────┼──►  ┌───────────────┐
                           │                │     │  Leaf          │
                           └───────────────┘     │  elements[8]  │ = "elem1000"
                                                  └───────────────┘
```

Why does this work? Because the trie is essentially a radix-32 number system.
Index 1000 in base 32 is (0, 31, 8) — and those are exactly the child indices
we follow at each level.

```
    More examples of bit partitioning:

    Index    Binary (15 bits)     Level 0    Level 1    Level 2
                                  >>10       >>5        >>0
    ──────   ─────────────────    ────────   ────────   ────────
    0        00000_00000_00000    0          0          0
    31       00000_00000_11111    0          0          31
    32       00000_00001_00000    0          1          0
    1023     00000_11111_11111    0          31         31
    1024     00001_00000_00000    1          0          0
    32767    11111_11111_11111    31         31         31
```

### Structural Sharing on Push/Set/Pop

When we modify an immutable list, we only copy the nodes along the path from root
to the affected leaf. All other nodes are shared between the old and new versions
via `Arc` (atomic reference-counted pointer).

```
    Example: set(index=33, "new_value") on a list with 64 elements

    BEFORE (original list):

    root ──► ┌──────────┐
             │ Internal  │
             │ [0]  [1]  │
             └──┼────┼───┘
                │    │
                ▼    ▼
            ┌──────┐┌──────┐
            │Leaf 0││Leaf 1│
            │ 0-31 ││32-63 │  ◄── element 33 is in Leaf 1
            └──────┘└──────┘

    AFTER set(33, "new_value"):

    old root ──► ┌──────────┐     new root ──► ┌──────────┐
                 │ Internal  │                  │ Internal' │  ◄── NEW node
                 │ [0]  [1]  │                  │ [0]  [1]  │
                 └──┼────┼───┘                  └──┼────┼───┘
                    │    │                         │    │
                    │    ▼                         │    ▼
                    │ ┌──────┐                     │ ┌──────┐
                    │ │Leaf 1│                     │ │Leaf 1'│  ◄── NEW leaf
                    │ │32-63 │                     │ │32-63  │     (elem 33 changed)
                    │ └──────┘                     │ └───────┘
                    │                              │
                    └──────────────┬───────────────┘
                                   │
                                   ▼
                              ┌──────┐
                              │Leaf 0│  ◄── SHARED (same Arc pointer)
                              │ 0-31 │
                              └──────┘

    Only 2 new nodes created (root + Leaf 1). Leaf 0 is shared.
    Both the old and new list are valid and independent.
```

### O(1) Clone via Arc Reference Counting

Cloning an immutable list is trivial — just increment the reference count on the
root node and the tail buffer. No data is copied. This is one of the most powerful
properties of persistent data structures.

```
    clone():

    original.root ──► ┌──────┐ ◄── clone.root
                      │ Node │
                      │ rc=2 │  ◄── reference count goes from 1 to 2
                      └──────┘

    original.tail ──► ┌──────┐ ◄── clone.tail
                      │ Vec  │
                      │ rc=2 │
                      └──────┘

    Cost: incrementing two atomic counters. That's it.
    No tree traversal. No element copying. O(1) time, O(1) space.
```

## Public API

All elements are `String` type for v1. This simplifies FFI marshaling across
language boundaries — strings are the universal data type that every language
handles natively. Future versions may support generic element types.

### Constructor Signatures

**`new()`** — Create an empty immutable list.

```
    Rust:       ImmutableList::new() -> ImmutableList
    Python:     ImmutableList() -> ImmutableList
    Ruby:       ImmutableList.new -> ImmutableList
    TypeScript: new ImmutableList() -> ImmutableList
    WASM:       ImmutableList.new() -> ImmutableList
```

Creates a list with `len = 0`, an empty root node, and an empty tail buffer.

**`from_slice(items)`** — Create an immutable list from an array of strings.

```
    Rust:       ImmutableList::from_slice(items: &[String]) -> ImmutableList
    Python:     ImmutableList.from_list(items: list[str]) -> ImmutableList
    Ruby:       ImmutableList.from_array(items) -> ImmutableList
    TypeScript: ImmutableList.fromArray(items: string[]) -> ImmutableList
    WASM:       ImmutableList.from_array(items: string[]) -> ImmutableList
```

Builds the trie bottom-up for efficiency. Equivalent to pushing each item in
order, but avoids repeated tail promotions by directly constructing leaf nodes.

### Element Access

**`get(index)`** — Retrieve the element at `index`. Returns `None`/`null`/`nil`
if the index is out of bounds.

```
    Rust:       fn get(&self, index: usize) -> Option<&str>
    Python:     def __getitem__(self, index: int) -> str     # raises IndexError
    Ruby:       def [](index) -> String | nil
    TypeScript: get(index: number): string | undefined
    WASM:       get(index: number): string | undefined
```

Python's `__getitem__` raises `IndexError` for out-of-bounds access (following
Python convention). All other languages return an option/nullable type.

**`len()`** — Return the number of elements in the list.

```
    Rust:       fn len(&self) -> usize
    Python:     def __len__(self) -> int                     # len(lst)
    Ruby:       def size -> Integer
    TypeScript: get length(): number
    WASM:       get length(): number
```

**`is_empty()`** — Return whether the list has zero elements.

```
    Rust:       fn is_empty(&self) -> bool
    Python:     def is_empty(self) -> bool
    Ruby:       def empty? -> true | false
    TypeScript: isEmpty(): boolean
    WASM:       isEmpty(): boolean
```

### Persistent Operations

All persistent operations return a **new** list. The original list is unchanged.

**`push(value)`** — Append an element to the end. Returns a new list.

```
    Rust:       fn push(&self, value: String) -> ImmutableList
    Python:     def push(self, value: str) -> ImmutableList
    Ruby:       def push(value) -> ImmutableList
    TypeScript: push(value: string): ImmutableList
    WASM:       push(value: string): ImmutableList
```

Fast path (~97% of calls): appends to tail buffer, O(1).
Slow path (every 32nd call): promotes tail into trie, then appends to new tail.

**`set(index, value)`** — Replace the element at `index`. Returns a new list.
Raises/panics if index is out of bounds.

```
    Rust:       fn set(&self, index: usize, value: String) -> ImmutableList
    Python:     def set(self, index: int, value: str) -> ImmutableList
    Ruby:       def set(index, value) -> ImmutableList
    TypeScript: set(index: number, value: string): ImmutableList
    WASM:       set(index: number, value: string): ImmutableList
```

Creates new nodes along the path from root to the affected leaf (path copying).
All other nodes are shared via `Arc`.

**`pop()`** — Remove the last element. Returns a tuple of (new list, removed value).
Raises/panics if the list is empty.

```
    Rust:       fn pop(&self) -> (ImmutableList, String)
    Python:     def pop(self) -> tuple[ImmutableList, str]
    Ruby:       def pop -> [ImmutableList, String]
    TypeScript: pop(): [ImmutableList, string]
    WASM:       pop(): [ImmutableList, string]
```

Fast path: removes from tail buffer, O(1).
Slow path (when tail becomes empty): demotes last trie leaf to become the new tail.

### Iteration and Conversion

**`iter()`** — Return an iterator over all elements in order.

```
    Rust:       fn iter(&self) -> impl Iterator<Item = &str>
    Python:     def __iter__(self) -> Iterator[str]
    Ruby:       def each(&block) -> self  (also include Enumerable)
    TypeScript: [Symbol.iterator](): Iterator<string>
    WASM:       toArray(): string[]  (iterators don't cross WASM boundary well)
```

**`to_vec()`** — Collect all elements into a plain array/vector.

```
    Rust:       fn to_vec(&self) -> Vec<String>
    Python:     def to_list(self) -> list[str]
    Ruby:       def to_a -> Array
    TypeScript: toArray(): string[]
    WASM:       toArray(): string[]
```

### Equality

**`eq(other)`** — Structural equality: two lists are equal if they have the same
length and all corresponding elements are equal.

```
    Rust:       impl PartialEq for ImmutableList   // a == b
    Python:     def __eq__(self, other) -> bool     # a == b
    Ruby:       def ==(other) -> true | false       # a == b
    TypeScript: equals(other: ImmutableList): boolean
    WASM:       equals(other: ImmutableList): boolean
```

Note: two lists can be structurally equal even if they have different internal
trie structures (e.g., one was built via repeated push, the other via from_slice).
Equality compares elements, not tree shape.

## Data Flow

### Push When Tail Has Room (Fast Path)

This is the common case — about 31 out of every 32 pushes.

```
    push("X") when tail has room:

    BEFORE                              AFTER
    ┌─────────────────┐                 ┌─────────────────┐
    │ root: ──────────►│ (same root)    │ root: ──────────►│ (same Arc)
    │ tail: ──────────►│ ["a","b","c"]  │ tail: ──────────►│ ["a","b","c","X"]
    │ len:  3          │                │ len:  4          │
    │ shift: 5         │                │ shift: 5         │
    └─────────────────┘                 └─────────────────┘

    Steps:
    1. Clone the tail buffer (Arc::make_mut or new Vec)
    2. Append "X" to the cloned tail
    3. Return new ImmutableList with same root, new tail, len+1

    Cost: one small array copy (up to 32 elements). No tree traversal.
```

### Push When Tail Is Full (Slow Path)

Every 32nd push triggers a tail promotion.

```
    push("X") when tail is full (32 elements):

    BEFORE:
    root ──► [Internal]               tail: [e32, e33, ..., e63]  (FULL)
                 │
                 ▼
              [Leaf 0: e0-e31]

    STEP 1: Promote current tail into trie as a new leaf node

    root' ──► [Internal']             (new internal node)
                 │    │
                 ▼    ▼
           [Leaf 0] [Leaf 1]          Leaf 1 = old tail (reused, not copied)
           (shared)  (promoted)

    STEP 2: Create new tail with the pushed element

    new tail: ["X"]                   (fresh buffer with 1 element)

    RESULT:
    ┌─────────────────────────────┐
    │ root:  ──► new Internal'     │
    │ tail:  ──► ["X"]             │
    │ len:   65                    │
    │ shift: 5                     │
    └─────────────────────────────┘

    Cost: O(log32 n) new internal nodes along the insertion path,
    plus the old tail is directly reused as a leaf (zero-copy promotion).
```

### Index Lookup via Bit Partitioning

```
    get(67) on a list with 96 elements:

    The list:
      root ──► [Internal: children[0], children[1], children[2]]
      tail: [e64, e65, e66, ..., e95]  (elements 64-95)

    Step 1: Is index 67 in the tail?
      tail_offset = len - tail.len() = 96 - 32 = 64
      67 >= 64?  YES — element 67 is in the tail.
      tail[67 - 64] = tail[3] = "e67"

    If index were 33 (in the trie):
      33 < 64 (tail_offset), so it's in the trie.

    Step 2: Descend the trie using bit partitioning.
      shift = 5 (depth 1 trie)
      child = (33 >> 5) & 0x1F = 1 & 31 = 1  →  root.children[1]
      elem  = (33 >> 0) & 0x1F = 1 & 31 = 1  →  leaf.elements[1] = "e33"

    Key insight: we check the tail FIRST. Most recent elements are in the
    tail, and recent elements are accessed most often (temporal locality).
```

### Set via Path Copying

```
    set(33, "NEW") on a list with 96 elements, depth-1 trie:

    BEFORE:
    root ──► [Internal]
                │    │    │
                ▼    ▼    ▼
           [Leaf0][Leaf1][Leaf2]

    Element 33 is in Leaf 1 (33 >> 5 = 1, 33 & 0x1F = 1).

    AFTER:
    new_root ──► [Internal']           ◄── NEW (copies child pointers)
                    │    │    │
                    │    │    │
                    ▼    ▼    ▼
              [Leaf0][Leaf1'][Leaf2]
              shared  NEW    shared

    Leaf1' is a copy of Leaf1 with elements[1] = "NEW".
    Internal' is a copy of Internal with children[1] = Arc::new(Leaf1').
    Leaf0 and Leaf2 are shared (same Arc, reference count incremented).

    Total new allocations: 1 internal node + 1 leaf node = 2 nodes.
    For a depth-4 trie: 4 internal nodes + 1 leaf = 5 nodes (still tiny).
```

### Clone Showing Arc Reference Counting

```
    let a = ImmutableList::from_slice(&["x", "y", "z"]);
    let b = a.clone();

    Memory layout:

    a.root ─┐
             ├──► Arc<Node> { ref_count: 2, data: Internal{...} }
    b.root ─┘

    a.tail ─┐
             ├──► Arc<Vec<String>> { ref_count: 2, data: ["x","y","z"] }
    b.tail ─┘

    a and b are fully independent logical lists.
    Mutating a (via push/set/pop) will clone-on-write only the
    affected path, leaving b's view of the data unchanged.

    let c = a.push("w");

    Now:
    a.root ──► Arc<Node> { ref_count: 2 }  ◄── b.root (still shared)
    a.tail ──► Arc<Vec>  { ref_count: 2 }  ◄── b.tail (still shared)
    c.root ──► Arc<Node> { ref_count: 2 }  (same as a.root — tail push didn't touch trie)
    c.tail ──► Arc<Vec>  { ref_count: 1, data: ["x","y","z","w"] }  (new tail)
```

## Internal Representation

```rust
struct ImmutableList {
    root: Arc<Node>,
    tail: Arc<Vec<String>>,
    len: usize,
    shift: u32,  // tree depth * 5 (0 = empty/tail-only, 5 = depth 1, 10 = depth 2, ...)
}

enum Node {
    Internal { children: [Option<Arc<Node>>; 32] },
    Leaf { elements: [Option<String>; 32] },
}
```

**Field explanations:**

- **`root`**: The root of the trie. For a list with <= 32 elements, the root is
  an empty internal node (all children `None`) because all elements live in the
  tail.

- **`tail`**: The tail buffer holding the last block of up to 32 elements. This is
  where `push` appends most of the time. It's a `Vec<String>` (not a fixed-size
  array) so we can track how many elements are in the tail without a separate
  counter.

- **`len`**: Total number of elements in the list (trie + tail combined).

- **`shift`**: Controls how many bits of the index to skip at the root level.
  `shift = depth * 5`. A shift of 0 means the trie is empty (all elements in
  tail). A shift of 5 means depth 1 (root's children are leaves). A shift of
  10 means depth 2 (root → internal → leaf). The trie grows deeper when the
  current depth can't accommodate all elements.

**Why `Arc<Vec<String>>` for the tail instead of `Arc<[Option<String>; 32]>`?**
Because the tail is frequently partially filled. A `Vec` tracks its own length,
so `tail.len()` tells us how many elements are in the tail without a separate
field. When the tail is promoted to a leaf, we convert it to the fixed-size
array format.

## Complexity Table

| Operation  | Time              | Space              | Notes                                  |
|------------|-------------------|--------------------|----------------------------------------|
| `new`      | O(1)              | O(1)               | Allocates root + empty tail            |
| `from_slice` | O(n)           | O(n)               | Bottom-up trie construction            |
| `push`     | O(1) amortized    | O(1) amortized     | ~97% fast path (tail append)           |
| `get`      | O(log32 n) ≈ O(1) | O(1)               | At most 6-7 levels for billions        |
| `set`      | O(log32 n)        | O(log32 n) new nodes | Path copying, shared siblings        |
| `pop`      | O(1) amortized    | O(1) amortized     | Mirror of push                         |
| `clone`    | O(1)              | O(1)               | Arc reference count increment          |
| `iter`     | O(n)              | O(log32 n) stack   | Visits every element exactly once      |
| `to_vec`   | O(n)              | O(n)               | Allocates new flat array               |
| `eq`       | O(n)              | O(1)               | Short-circuits on length mismatch      |
| `len`      | O(1)              | O(1)               | Stored field                           |
| `is_empty` | O(1)              | O(1)               | Checks len == 0                        |

**Why O(log32 n) is effectively O(1):** For a list with 1 billion elements,
log32(1,000,000,000) ≈ 6. The maximum depth for any list that fits in memory
is 7 or 8 levels. In practice, `get` and `set` are constant-time operations
with a small constant factor.

**Amortized analysis for push:** 31 out of 32 pushes are O(1) tail appends.
The 32nd push promotes the tail to the trie, which is O(log32 n). Amortized
over 32 operations: (31 * O(1) + 1 * O(log32 n)) / 32 = O(1) amortized.

## Test Strategy

### Correctness Tests

1. **Empty list** — `new()` produces len=0, is_empty=true, get(0) returns None.
2. **Single element** — push one, get(0) works, len=1, pop returns it.
3. **Sequential push/get** — push 100 elements, verify get(i) == expected for all i.
4. **Set correctness** — set(i, new_val) returns list where only index i changed.
5. **Pop correctness** — pop returns last element, new list has len-1.
6. **Iteration order** — iter() yields elements in insertion order.
7. **to_vec round-trip** — from_slice(items).to_vec() == items.
8. **Equality** — lists with same elements are equal, different elements are not.

### Structural Sharing Verification

9. **Push preserves original** — after `b = a.push("x")`, verify `a` is unchanged.
10. **Set preserves original** — after `b = a.set(0, "new")`, verify `a[0]` is old value.
11. **Pop preserves original** — after `(b, _) = a.pop()`, verify `a.len()` unchanged.
12. **Clone independence** — clone, modify clone, verify original unchanged.
13. **Deep sharing** — push 10,000 elements, clone, push 1 more to clone.
    Verify both lists are valid and independent.

### Boundary and Edge Cases

14. **Exactly 32 elements** — fills the tail exactly, no trie nodes needed.
15. **33 elements** — triggers first tail promotion, creates first trie leaf.
16. **1024 elements** — fills depth-1 trie completely (32 leaves x 32 elements).
17. **1025 elements** — triggers trie depth increase from 1 to 2.
18. **32,768 elements** — fills depth-2 trie completely.
19. **100,000 elements** — stress test for depth-3 trie, push/get/set/pop all work.
20. **Pop to empty** — push 100 elements, pop all 100, verify empty.
21. **Alternating push/pop** — push 5, pop 3, push 5, pop 3, verify state.

### Performance Smoke Tests

22. **Push throughput** — 1,000,000 pushes complete in reasonable time.
23. **Get random access** — 1,000,000 random gets on a large list.
24. **Clone is O(1)** — cloning a 1,000,000-element list is near-instant.

### FFI Bridge Tests (per language)

25. **Round-trip** — create in Rust, expose to Python/Ruby/TS/WASM, read back.
26. **String encoding** — non-ASCII strings (emoji, Unicode) survive the bridge.
27. **Error handling** — out-of-bounds get/set/pop produce appropriate errors.
28. **Memory safety** — no leaks under repeated create/modify/drop cycles.

## Future Extensions

- **Generic element types** — support integers, floats, booleans, nested lists.
  Requires a type-tagged union or generic specialization in the Rust core.

- **RRB-tree (Relaxed Radix Balanced tree)** — extends the 32-way trie to support
  O(log n) concatenation and slicing. The current design requires O(n) for concat.
  RRB-trees allow variable-width nodes (not strictly 32) to enable efficient
  structural sharing during concat operations.

- **Transient (mutable) mode** — a temporary mutable view of the list for batch
  operations. Instead of creating a new list for each of 1000 pushes, convert to
  transient, do all 1000 pushes with in-place mutation, then convert back to
  persistent. This is how Clojure's `transient` and `persistent!` work.

- **ImmutableMap (HAMT)** — Hash Array Mapped Trie. Same 32-way branching idea,
  but keyed by hash rather than index. Structural sharing gives O(log32 n)
  insert/lookup/delete with immutability.

- **ImmutableSet** — built on top of ImmutableMap (a map where values are ignored),
  providing set operations (union, intersection, difference) with structural sharing.

- **Serialization** — efficient binary serialization format that preserves structural
  sharing (e.g., for sending persistent data structures over the network or persisting
  to disk).
