# DT19 — Hash Set

## Overview

A **hash set** is a collection that stores unique elements and answers one question
instantly: "Is this element in the set?"

It is built directly on top of DT18 (hash map). The insight is simple: a hash map
stores key→value pairs, but for set membership we only care whether a key *exists*
— the value is irrelevant. So we use the hash map's keys as the set's elements and
discard the values entirely (or store a zero-byte sentinel).

```
Hash Map (DT18):          Hash Set (DT19):
┌────────┬────────┐       ┌────────┐
│  key   │ value  │       │  key   │
├────────┼────────┤       ├────────┤
│ "alice"│  42    │       │ "alice"│
│ "bob"  │  7     │  →    │ "bob"  │
│ "carol"│  99    │       │ "carol"│
└────────┴────────┘       └────────┘
  two columns               one column
```

Beyond raw membership, sets support rich **set algebra** — union, intersection,
difference — the same operations you learned in middle-school Venn diagrams.
These operations are the reason sets are a first-class data structure in Python,
Ruby, Java, Go, and most standard libraries.

## Layer Position

```
DT17: hash-functions    ← hashing primitives (MurmurHash, FNV-1a, xxHash)
DT18: hash-map          ← key→value mapping, collision handling
DT19: hash-set          ← [YOU ARE HERE]  (hash map with keys-only)
  │
  ├── compare with:
  │     DT08: avl-tree      (sorted set, O(log n) ops, range queries)
  │     DT09: red-black-tree (sorted set used by Java TreeSet, C++ std::set)
  │     DT10: treap          (randomized sorted set)
  │
  └── used by:
        DT21: hyperloglog    (approximate set membership counting)
        DT22: bloom-filter   (probabilistic set membership)
        DT25: mini-redis     (SADD/SMEMBERS/SINTER commands)
```

**Depends on:** DT17 (hash functions), DT18 (hash map internals).
**Contrasts with:** DT08/DT09 for sorted sets with range queries.
**Used by:** DT25 mini-redis (Redis SET commands), graph algorithms (visited sets),
deduplication pipelines, cache eviction lists.

## Concepts

### The Fundamental Set Problem

Suppose you are processing a web server log and want to count the number of
*unique* visitors. The log has 10 million lines but many repeat IPs:

```
192.168.1.1  GET /index.html
10.0.0.5     GET /about.html
192.168.1.1  GET /contact.html   ← duplicate
172.16.0.99  GET /index.html
10.0.0.5     POST /login          ← duplicate
...
```

With a list, you'd check every previous entry for duplicates: O(n) per insert,
O(n²) total. With a sorted list (binary search), O(log n) per insert but O(n)
for rebalancing. With a hash set: O(1) per insert and O(1) for membership check.

### How a Hash Set Works Internally

A hash set is a hash map (DT18) where we simply never store the value:

```
add("alice"):
  slot = hash("alice") % capacity           # e.g., slot 3
  buckets[3] = Node(key="alice", next=None) # store key, no value

contains("alice"):
  slot = hash("alice") % capacity           # same slot 3
  walk chain at slot 3, looking for "alice" # found → True

contains("dave"):
  slot = hash("dave") % capacity            # e.g., slot 7
  walk chain at slot 7, looking for "dave"  # not found → False
```

The implementation literally *is* the hash map with the value field removed.
In Python, `set` is implemented this way in CPython. In Rust, `HashSet<T>` is
`HashMap<T, ()>` — the `()` (unit type) takes zero bytes.

### Set Algebra: Venn Diagrams in Code

The real power of sets is combining them. Here is what each operation means:

```
Set A = {1, 2, 3, 4, 5}
Set B = {3, 4, 5, 6, 7}

Union  A ∪ B:              everything in A or B (or both)
  ┌─────────────┐
  │  1   2  │3 4│  6   7  │
  │     A   │ ∩ │    B    │
  └─────────────┘
  result: {1, 2, 3, 4, 5, 6, 7}

Intersection  A ∩ B:       only what both share
  ┌─────────────┐
  │  1   2  │3 4│  6   7  │
  │         │ ∩ │         │
  └─────────────┘
  result: {3, 4, 5}

Difference  A − B:         in A but NOT in B
  ┌─────────────┐
  │  1   2  │   │         │
  │    A    │   │         │
  └─────────────┘
  result: {1, 2}

Symmetric Difference  A △ B:  in A or B but NOT both
  ┌─────────────┐
  │  1   2  │   │  6   7  │
  │         │   │         │
  └─────────────┘
  result: {1, 2, 6, 7}
```

### Subset and Superset Checks

```
A = {1, 2, 3}
B = {1, 2, 3, 4, 5}

A ⊆ B (A is subset of B)?      Every element of A is in B → True
B ⊇ A (B is superset of A)?    B contains all of A → True
A ⊆ A?                         A set is always a subset of itself → True
{} ⊆ A?                        Empty set is subset of everything → True

A and C = {10, 20} are disjoint?  No elements in common → True
A and D = {3, 100} are disjoint?  Share element 3 → False
```

### Hash Set vs Tree Set: When to Use Which

This is one of the most common interview questions. The answer depends entirely
on what you need:

```
Operation          Hash Set (DT19)     Tree Set (DT08/DT09)
-----------------  ------------------  --------------------
add(x)             O(1) average        O(log n) always
contains(x)        O(1) average        O(log n) always
remove(x)          O(1) average        O(log n) always
union(A, B)        O(|A| + |B|)        O(|A| + |B|)  ← same!
intersection       O(min(|A|, |B|))    O(|A| + |B|)
iteration order    undefined / random  sorted always
range query        O(n) full scan      O(log n + k)
min / max          O(n)                O(log n)
successor(x)       O(n)                O(log n)
memory per element ~40–64 bytes        ~48–80 bytes
worst-case lookup  O(n) (bad hash)     O(log n) always
```

**Choose Hash Set when:**
- You need fast membership checks (caches, visited sets in BFS/DFS)
- You're doing set algebra and don't need sorted output
- You don't need range queries ("all elements between 5 and 10")
- The element type has a good hash function

**Choose Tree Set when:**
- You need sorted iteration (producing sorted output without re-sorting)
- You need range queries (find all emails starting with "a" through "m")
- You need min/max/successor/predecessor operations
- Worst-case guarantees matter (real-time systems)

### Real-World Use Cases

**Web crawlers (visited set):** BFS graph traversal needs to track visited URLs.
A hash set of URLs gives O(1) lookup so we never re-crawl a page.

**Database query optimization:** "Has this row been emitted?" in a join uses
a hash set as the probe table. This is exactly how hash joins work.

**Spell checking:** Store dictionary words in a hash set. Check any word in O(1).
(A bloom filter from DT22 is even more space-efficient for this use case.)

**Access control:** "Is this user in the admin group?" — hash set lookup.

**Deduplication:** Stream of events, emit each unique event exactly once.

## Representation

A hash set wraps a hash map. Internally, the only storage is the array of
buckets (open addressing) or bucket chains (separate chaining). There is no
value array at all.

```
HashSet {
    map: HashMap<K, ()>   # reuse all of DT18's machinery
}

# Or, if implementing from scratch with open addressing:
HashSet {
    buckets: Array[Slot]    # each slot: EMPTY | DELETED | Occupied(key)
    size: int               # number of live elements
    capacity: int           # length of buckets array
    load_factor: float      # default 0.75; resize when size/capacity > load_factor
}

Slot = EMPTY | DELETED | Occupied(key)
```

The `DELETED` tombstone is needed for open addressing: if we mark a slot as
EMPTY when removing, we break probe chains for elements that were inserted
after the removed element. Tombstones preserve probe chain integrity while
marking slots as available for future insertions.

### Memory Layout (Open Addressing)

```
capacity = 8, size = 3, elements = {"alice", "bob", "carol"}

Index:    0       1       2       3       4       5       6       7
        ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
bucket: │ EMPTY │"bob"  │ EMPTY │"alice"│ EMPTY │"carol"│ EMPTY │ EMPTY │
        └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘
                  ↑               ↑               ↑
              hash("bob")=1   hash("alice")=3  hash("carol")=5
              (mod 8)         (mod 8)           (mod 8)
```

## Algorithms (Pure Functions)

All algorithms delegate to the underlying hash map. We describe them here
to make the set semantics explicit.

### add(set, element) → new set

```
add(set, element):
    # Delegate: insert element into the underlying map with unit value
    new_map = hash_map_put(set.map, key=element, value=())
    return HashSet(map=new_map)
```

Time: O(1) average, O(n) worst case (resize or collision chain).

### contains(set, element) → bool

```
contains(set, element):
    return hash_map_get(set.map, key=element) is not None
```

Time: O(1) average.

### remove(set, element) → new set

```
remove(set, element):
    new_map = hash_map_delete(set.map, key=element)
    return HashSet(map=new_map)
```

Time: O(1) average.

### union(set1, set2) → new set

Strategy: start with the larger set (fewer insertions), add all elements from
the smaller set. This is O(|smaller|) insertions into a pre-built structure.

```
union(set1, set2):
    larger, smaller = (set1, set2) if size(set1) >= size(set2) else (set2, set1)
    result = copy(larger)
    for element in to_list(smaller):
        result = add(result, element)
    return result
```

Time: O(|set1| + |set2|) — must at minimum iterate both sets to build result.

### intersection(set1, set2) → new set

Strategy: iterate the *smaller* set, check membership in the larger. Only
elements in both sets are added to the result.

```
intersection(set1, set2):
    smaller, larger = (set1, set2) if size(set1) <= size(set2) else (set2, set1)
    result = empty_set()
    for element in to_list(smaller):
        if contains(larger, element):
            result = add(result, element)
    return result
```

Time: O(min(|set1|, |set2|)) iterations × O(1) lookup = O(min(|A|, |B|)).
This is *faster* than tree set intersection which is O(|A| + |B|).

### difference(set1, set2) → new set

Elements in set1 but NOT in set2.

```
difference(set1, set2):
    result = empty_set()
    for element in to_list(set1):
        if not contains(set2, element):
            result = add(result, element)
    return result
```

Time: O(|set1|) — iterate set1, O(1) membership check in set2.

### symmetric_difference(set1, set2) → new set

Elements in either set but not both. Equivalent to (A − B) ∪ (B − A),
but can be computed more efficiently in one pass:

```
symmetric_difference(set1, set2):
    result = empty_set()
    for element in to_list(set1):
        if not contains(set2, element):
            result = add(result, element)
    for element in to_list(set2):
        if not contains(set1, element):
            result = add(result, element)
    return result
```

Time: O(|set1| + |set2|).

### is_subset(set1, set2) → bool

Every element of set1 must be in set2. Short-circuit on first miss.

```
is_subset(set1, set2):
    if size(set1) > size(set2):
        return False   # fast reject: set1 can't be subset if it's larger
    for element in to_list(set1):
        if not contains(set2, element):
            return False
    return True
```

Time: O(|set1|) in worst case (all elements present).

### is_superset(set1, set2) → bool

set1 is superset iff set2 is subset of set1.

```
is_superset(set1, set2):
    return is_subset(set2, set1)
```

### is_disjoint(set1, set2) → bool

No elements in common. Iterate smaller, check in larger.

```
is_disjoint(set1, set2):
    smaller, larger = (set1, set2) if size(set1) <= size(set2) else (set2, set1)
    for element in to_list(smaller):
        if contains(larger, element):
            return False
    return True
```

Time: O(min(|set1|, |set2|)) with early exit.

## Public API

Python-style pseudocode. The `@` symbol denotes the pure-functional result.

```python
class HashSet:
    """
    An unordered collection of unique elements.
    All mutating operations return a NEW set (persistent/functional style).
    """

    def __init__(self) -> "HashSet":
        """Create an empty set."""

    @staticmethod
    def from_list(elements: list) -> "HashSet":
        """Build a set from a list, discarding duplicates."""

    def add(self, element) -> "HashSet":
        """Return a new set with element included."""

    def remove(self, element) -> "HashSet":
        """Return a new set without element. No error if absent."""

    def discard(self, element) -> "HashSet":
        """Alias for remove (matches Python API)."""

    def contains(self, element) -> bool:
        """Return True if element is in the set. O(1) average."""

    def size(self) -> int:
        """Number of distinct elements."""

    def is_empty(self) -> bool:
        """True if size == 0."""

    def to_list(self) -> list:
        """Return all elements as a list (order is undefined)."""

    # --- Set algebra ---

    def union(self, other: "HashSet") -> "HashSet":
        """A ∪ B — all elements in either set."""

    def intersection(self, other: "HashSet") -> "HashSet":
        """A ∩ B — only elements in both sets."""

    def difference(self, other: "HashSet") -> "HashSet":
        """A − B — elements in self but not other."""

    def symmetric_difference(self, other: "HashSet") -> "HashSet":
        """A △ B — elements in one but not both."""

    # --- Relational checks ---

    def is_subset(self, other: "HashSet") -> bool:
        """True if every element of self is in other (self ⊆ other)."""

    def is_superset(self, other: "HashSet") -> bool:
        """True if self contains all elements of other (self ⊇ other)."""

    def is_disjoint(self, other: "HashSet") -> bool:
        """True if self and other share no elements."""

    def equals(self, other: "HashSet") -> bool:
        """True if self and other contain exactly the same elements."""
```

## Composition Model

Hash set is implemented differently across languages. In languages with
**inheritance**, we subclass or wrap the hash map. In languages with
**composition** (the more common and idiomatic approach), we embed a hash map.

### Python / Ruby / TypeScript — Wrapper Class (Inheritance or Delegation)

In Python, the built-in `set` already wraps a dict internally. For our
educational implementation:

```python
# Python: composition — wrap HashMap from DT18
class HashSet:
    def __init__(self):
        self._map = HashMap()   # DT18 hash map

    def add(self, element):
        new_set = HashSet()
        new_set._map = self._map.put(element, None)
        return new_set

    def contains(self, element) -> bool:
        return self._map.get(element) is not None
```

```typescript
// TypeScript: generic wrapper
class HashSet<T> {
    private readonly map: HashMap<T, null>; // DT18

    constructor(map?: HashMap<T, null>) {
        this.map = map ?? new HashMap();
    }

    add(element: T): HashSet<T> {
        return new HashSet(this.map.put(element, null));
    }

    contains(element: T): boolean {
        return this.map.get(element) !== undefined;
    }
}
```

### Rust — Newtype Wrapper (Zero-Cost Abstraction)

```rust
// Rust: HashSet<T> is literally HashMap<T, ()>
pub struct HashSet<T> {
    map: HashMap<T, ()>,   // () = unit type, zero bytes
}

impl<T: Hash + Eq> HashSet<T> {
    pub fn insert(&mut self, value: T) -> bool {
        self.map.insert(value, ()).is_none()
    }
    pub fn contains(&self, value: &T) -> bool {
        self.map.contains_key(value)
    }
}
// The standard library std::collections::HashSet does exactly this.
```

### Go — Struct with Embedded Map

```go
// Go: idiomatic map[T]struct{} pattern
type HashSet[T comparable] struct {
    m map[T]struct{}
}

func (s HashSet[T]) Add(elem T) HashSet[T] {
    next := make(map[T]struct{}, len(s.m)+1)
    for k := range s.m {
        next[k] = struct{}{}
    }
    next[elem] = struct{}{}
    return HashSet[T]{m: next}
}

func (s HashSet[T]) Contains(elem T) bool {
    _, ok := s.m[elem]
    return ok
}
```

### Elixir — MapSet from Standard Library

```elixir
# Elixir: MapSet is the standard hash set, backed by a Map
# For our implementation, we wrap a plain map:
defmodule HashSet do
  defstruct map: %{}

  def new(), do: %HashSet{}

  def add(%HashSet{map: m} = _set, element) do
    %HashSet{map: Map.put(m, element, true)}
  end

  def contains?(%HashSet{map: m}, element) do
    Map.has_key?(m, element)
  end

  def union(%HashSet{map: m1}, %HashSet{map: m2}) do
    %HashSet{map: Map.merge(m1, m2)}
  end
end
```

### Lua — Table-as-Set Idiom

```lua
-- Lua: use table with keys as elements, true as dummy value
local HashSet = {}
HashSet.__index = HashSet

function HashSet.new()
    return setmetatable({_t = {}, _size = 0}, HashSet)
end

function HashSet:add(element)
    local next = HashSet.new()
    for k in pairs(self._t) do next:_raw_add(k) end
    next:_raw_add(element)
    return next
end

function HashSet:contains(element)
    return self._t[element] == true
end
```

## Test Strategy

### Unit Tests for Core Operations

```python
# Membership basics
s = HashSet.from_list([1, 2, 3])
assert s.contains(1) == True
assert s.contains(4) == False
assert s.size() == 3

# Duplicates are ignored
s = HashSet.from_list([1, 1, 2, 2, 3])
assert s.size() == 3

# add is pure (original unchanged)
s1 = HashSet.from_list([1, 2])
s2 = s1.add(3)
assert s1.size() == 2    # original untouched
assert s2.size() == 3
```

### Set Algebra Correctness

```python
A = HashSet.from_list([1, 2, 3, 4, 5])
B = HashSet.from_list([3, 4, 5, 6, 7])

# Union
u = A.union(B)
assert set(u.to_list()) == {1, 2, 3, 4, 5, 6, 7}

# Intersection
i = A.intersection(B)
assert set(i.to_list()) == {3, 4, 5}

# Difference
d = A.difference(B)
assert set(d.to_list()) == {1, 2}

# Symmetric difference
sd = A.symmetric_difference(B)
assert set(sd.to_list()) == {1, 2, 6, 7}
```

### Relational Tests

```python
A = HashSet.from_list([1, 2, 3])
B = HashSet.from_list([1, 2, 3, 4, 5])
C = HashSet.from_list([10, 20])

assert A.is_subset(B) == True
assert B.is_superset(A) == True
assert A.is_subset(A) == True     # reflexive
assert A.is_disjoint(C) == True
assert A.is_disjoint(B) == False  # share 1, 2, 3

# Empty set is subset of everything
empty = HashSet()
assert empty.is_subset(A) == True
assert empty.is_subset(empty) == True
```

### Edge Cases

```python
# Empty set operations
empty = HashSet()
assert empty.union(A).equals(A)
assert empty.intersection(A).equals(empty)
assert empty.difference(A).equals(empty)
assert A.difference(empty).equals(A)

# Single element sets
s = HashSet.from_list(["x"])
assert s.contains("x") == True
assert s.remove("x").is_empty() == True

# Hashable types: strings, ints, tuples
s = HashSet.from_list(["hello", 42, (1, 2)])
assert s.size() == 3
```

### Performance Tests

```python
import time

# 1 million elements — membership check should be sub-millisecond
big = HashSet.from_list(range(1_000_000))
start = time.perf_counter()
for i in range(1000):
    big.contains(i * 999)
elapsed = time.perf_counter() - start
assert elapsed < 0.01   # 1000 lookups in < 10ms

# Intersection of two 100k sets should complete in < 1s
A = HashSet.from_list(range(0, 100_000))
B = HashSet.from_list(range(50_000, 150_000))
result = A.intersection(B)
assert result.size() == 50_000
```

## Future Extensions

**Persistent (Immutable) Sets:** The functional API above returns new sets.
A persistent hash array mapped trie (HAMT) — used by Clojure and Scala —
achieves O(log₃₂ n) structural sharing so copying is O(1) instead of O(n).

**Concurrent Sets:** For multi-threaded programs, Java's `ConcurrentHashMap`
backing a set supports lock-striped concurrent access. Lock-free variants
exist using CAS (compare-and-swap) primitives.

**Ordered Hash Sets:** Python's `dict` (3.7+) preserves insertion order.
Building a hash set on top gives an insertion-ordered set — useful for
deduplication while preserving order (e.g., removing duplicate lines from
a log while keeping the first occurrence).

**Counting Multiset (Bag):** Extend the hash map value from `()` to `int`
to count how many times each element was added. This is the "multiset" or
"bag" data structure — a set that tracks duplicates. Directly related to
DT22 (counting bloom filter).

**Probabilistic Membership (DT22):** When the set contains billions of
elements and memory is constrained, replace the hash set with a bloom filter.
You trade perfect accuracy for a 1000× memory reduction. This is the tradeoff
that DT22 explores in depth.
