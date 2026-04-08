# DT18 — Hash Map

## Overview

A **hash map** (also called a hash table, dictionary, or associative array)
stores key-value pairs and supports O(1) average-case lookup, insertion, and
deletion. It is arguably the single most useful data structure in everyday
programming — Python's `dict`, JavaScript's `Object`/`Map`, Ruby's `Hash`,
Go's `map`, and Java's `HashMap` are all hash maps.

The core idea: given a key, compute an integer index using a hash function
(DT17), then store the value at that index in an underlying array. Lookup is
fast because array indexing is O(1). The tricky part is handling **collisions**
— two keys that hash to the same index — and **resizing** the array as the map
fills up.

This spec implements a hash map from scratch in two collision strategies:
**separate chaining** and **open addressing (linear probing)**. Understanding
both prepares you to reason about performance trade-offs in production systems.

## Layer Position

```
DT17: hash-functions  ← direct dependency (compute bucket index)
  └── DT18: hash-map  ← [YOU ARE HERE]
        └── DT19: hash-set   (hash map with no values, just keys)

DT25: mini-redis       (uses hash maps for key-value storage)
```

**Depends on:** DT17 (hash functions: `fnv1a_32`, `siphash_2_4`).
**Extended by:** DT19 (hash set: a hash map that stores keys without values).
**Used by:** DT25 (mini-redis), every application ever written.

## Concepts

### The Core Insight: Array + Hash = O(1)

Suppose you have an array of 16 slots and a function that converts any key
into a number from 0 to 15. You can store and retrieve any key-value pair in
constant time:

```
store(key="cat", value=1):
  index = hash("cat") % 16 = 3     ← pick a slot
  array[3] = ("cat", 1)

retrieve(key="cat"):
  index = hash("cat") % 16 = 3     ← same slot
  return array[3].value            → 1
```

No searching required — we go directly to the right slot. This works as long
as every key maps to a different slot. When two keys map to the same slot,
that is a **collision**, and we need a strategy to handle it.

### Strategy 1: Separate Chaining

Each slot in the array holds a **linked list** (or small array) of (key, value)
pairs. When a collision occurs, the new pair is appended to the list at that
slot. Lookup scans the list at the target slot.

```
Array of 10 slots:

slot 0: []
slot 1: []
slot 2: [("banana", 3)]
slot 3: [("cat", 1) → ("car", 2)]    ← collision: cat and car both hash to 3
slot 4: []
slot 5: [("dog", 5)]
slot 6: []
slot 7: []
slot 8: [("elephant", 8)]
slot 9: []
```

"cat" and "car" both hash to slot 3. The list at slot 3 holds both.
Looking up "car" requires hashing "car" to slot 3, then scanning the list
`[("cat", 1), ("car", 2)]` until we find "car". If the load factor is low
(few items per slot), the list is almost always length 0 or 1 — O(1) average.

**Advantages:**
- Simple to implement
- Deletion is easy (remove from the linked list)
- Handles any load factor (list can grow arbitrarily)
- Cache-friendly for short chains (1-2 elements fit in one cache line)

**Disadvantages:**
- Pointer overhead (each list node requires a heap allocation)
- Poor cache behavior when chains grow long (pointer chasing across memory)
- Higher memory overhead than open addressing for low-load-factor tables

### Strategy 2: Open Addressing (Linear Probing)

Everything is stored directly in the flat array — no linked lists, no heap
allocations per entry. When a collision occurs, **probe** to the next slot.

**Linear probing** tries slot `h+1`, then `h+2`, then `h+3`, etc. (wrapping
around at the end of the array):

```
Inserting "cat" (hash → 3) and "car" (hash → 3) into a 10-slot array:

Insert "cat" → hash=3, slot 3 empty → store at slot 3
Insert "car" → hash=3, slot 3 occupied by "cat"
            → try slot 4, slot 4 empty → store at slot 4

Array:
slot 0: empty
slot 1: empty
slot 2: empty
slot 3: ("cat", 1)
slot 4: ("car", 2)     ← probed one slot forward
slot 5: empty
...
```

Lookup for "car":
```
hash("car") = 3 → check slot 3 → key is "cat", not "car"
              → check slot 4 → key is "car" ✓ found
```

**The clustering problem:**
When many keys hash to nearby slots, they form a **cluster** — a long run of
occupied slots. New keys must probe further and further to find empty slots.
The cluster grows, making future insertions and lookups slower.

```
Dense cluster (bad):
slot 3: ("cat", 1)
slot 4: ("car", 2)
slot 5: ("cab", 3)
slot 6: ("can", 4)
slot 7: ("cap", 5)

A new key with hash 3 must probe 5 slots forward!
```

**Mitigations:**
- **Quadratic probing**: try offsets 1², 2², 3² instead of 1, 2, 3. Spreads
  probes more, reduces primary clustering. But can miss slots (cycle).
- **Double hashing**: step size = `h2(key)` (a second hash function). Each
  key has its own unique probe sequence, dramatically reducing clustering.
  Used by Python's `dict`.
- **Robin Hood hashing**: when inserting, if the new key has probed further
  than the key currently at a slot, steal the slot and re-insert the
  displaced key. This equalizes probe distances.

**Advantages:**
- No pointer overhead — all data in one flat array
- Excellent cache behavior (sequential memory access)
- Faster than chaining at low load factors

**Disadvantages:**
- Deletion is tricky (see Tombstones below)
- Degrades significantly above load factor ~0.7
- Requires careful sizing (table must stay sparse)

### Load Factor and Resizing

The **load factor** is:

```
α = n / capacity

where n        = number of items currently stored
      capacity = total number of array slots
```

As α increases, performance degrades:

```
Expected probe length for linear probing (from Knuth's analysis):
  α = 0.50 → 1.5 probes average for successful lookup
  α = 0.75 → 2.5 probes average
  α = 0.90 → 5.5 probes average
  α = 0.99 → 50.5 probes average!
```

Most hash map implementations **resize** when α exceeds a threshold:
- Separate chaining: resize at α > 1.0 (some allow α up to 3-4)
- Open addressing: resize at α > 0.75 (Python uses 0.666, Java uses 0.75)

**Resize algorithm:**
```
1. Allocate new array of size capacity * 2
2. For each (key, value) in old array:
     new_index = hash(key) % new_capacity
     probe until empty slot in new array
     store (key, value) at new_index
3. Discard old array
```

This is O(n) — each element is re-hashed once. But it happens infrequently
(only when the table doubles), so the amortized cost per insertion is O(1).

**Amortized analysis (why O(1) amortized):**

Imagine a table that starts at capacity 1 and doubles every time it fills up:
```
Insert 1 element → no resize
Insert 2nd       → resize: re-hash 1 element
Insert 3rd, 4th  → resize: re-hash 2 elements
Insert 5–8       → resize: re-hash 4 elements
Insert 9–16      → resize: re-hash 8 elements
...
```

Total re-hash work after n insertions = 1 + 2 + 4 + 8 + ... + n/2 = O(n).
Divide by n insertions: O(1) amortized per insertion. Each element is
"re-hashed" at most O(log n) times during the lifetime of the table.

### Deletion in Open Addressing: Tombstones

You cannot simply clear a slot in open addressing. Consider:

```
Insert "cat" (hash=3) → slot 3
Insert "car" (hash=3) → slot 4 (probed)

Now delete "cat" (slot 3):
  If we clear slot 3 → slot 3 is now empty

Later lookup for "car":
  hash("car") = 3 → check slot 3 → EMPTY → return "not found"!
  WRONG: "car" is still in slot 4, but we stopped probing at the empty slot.
```

**Solution: Tombstone markers.** When deleting, instead of clearing the slot,
mark it as a TOMBSTONE ("deleted but once occupied"):

```
slot 3: TOMBSTONE   ← was "cat", now deleted
slot 4: ("car", 2)  ← still here
```

During lookup, skip TOMBSTONES (continue probing). During insertion, fill
TOMBSTONES (reuse the slot). This maintains probe chain integrity.

**Downside:** over time, tombstones accumulate and slow down lookups. Solution:
periodically rebuild the table (re-hash all live entries, discarding tombstones).
This is done automatically during resize.

### A Complete Worked Example

Insert 8 keys into a hash map (capacity=8, strategy=chaining):

```
Keys and hash values (using fnv1a_32 mod 8):
  "apple"  → hash mod 8 = 5
  "banana" → hash mod 8 = 2
  "cherry" → hash mod 8 = 7
  "date"   → hash mod 8 = 2   ← collision with "banana"
  "elderberry" → 3
  "fig"    → 6
  "grape"  → 1
  "honeydew" → 5              ← collision with "apple"

After inserting all 8 keys (load factor = 8/8 = 1.0):

slot 0: []
slot 1: [("grape", ...)]
slot 2: [("banana", ...) → ("date", ...)]
slot 3: [("elderberry", ...)]
slot 4: []
slot 5: [("apple", ...) → ("honeydew", ...)]
slot 6: [("fig", ...)]
slot 7: [("cherry", ...)]
```

Load factor hits 1.0 — trigger resize to capacity=16.

```
After resize to capacity=16 (re-hash all keys mod 16):
  "apple"      → 13
  "banana"     → 2
  "cherry"     → 15
  "date"       → 10
  "elderberry" → 11
  "fig"        → 6
  "grape"      → 9
  "honeydew"   → 5

slot 2:  [("banana", ...)]     ← no more collision!
slot 5:  [("honeydew", ...)]
slot 6:  [("fig", ...)]
slot 9:  [("grape", ...)]
slot 10: [("date", ...)]
slot 11: [("elderberry", ...)]
slot 13: [("apple", ...)]
slot 15: [("cherry", ...)]

Load factor = 8/16 = 0.5. All chains length 1. O(1) for all lookups.
```

## Representation

### Chaining variant

```
ChainedEntry:
  key:   any hashable type
  value: any type

ChainedBucket:
  entries: list[ChainedEntry]

ChainedHashMap:
  buckets:  list[ChainedBucket]   # length = capacity
  size:     int                   # number of (key, value) pairs
  capacity: int
  hash_fn:  HashFunction          # from DT17
```

### Open addressing variant

```
Slot state: EMPTY | TOMBSTONE | OCCUPIED

OAEntry:
  key:   any hashable type
  value: any type
  state: SlotState

OpenAddressHashMap:
  slots:    list[OAEntry | None]  # length = capacity
  size:     int                   # number of live entries
  capacity: int
  hash_fn:  HashFunction
```

## Algorithms (Pure Functions)

### `new_map(capacity=16, strategy) → HashMap`

```
buckets = [empty_bucket() for _ in range(capacity)]
return HashMap(buckets, size=0, capacity=capacity)
```

### `_bucket_index(map, key) → int`

```
return hash_fn(serialize(key)) % map.capacity
```

### `set(map, key, value) → HashMap`  (functional: returns new map)

**Chaining:**
```
idx = _bucket_index(map, key)
bucket = map.buckets[idx]
for i, entry in enumerate(bucket.entries):
    if entry.key == key:
        new_bucket = replace entry[i] with (key, value)
        return updated map (same size)
# not found: append new entry
new_bucket = bucket.entries + [(key, value)]
new_map = map with buckets[idx] = new_bucket, size = map.size + 1
if load_factor(new_map) > 1.0:
    return _resize(new_map)
return new_map
```

**Open addressing:**
```
idx = _bucket_index(map, key)
first_tombstone = None
for probe in 0..capacity:
    i = (idx + probe) % capacity
    if slots[i].state == EMPTY:
        # insert here (or at first tombstone)
        insert_at = first_tombstone if first_tombstone is not None else i
        slots[insert_at] = OAEntry(key, value, OCCUPIED)
        new_size = map.size + 1
        if load_factor > 0.75: return _resize(new_map)
        return new_map
    elif slots[i].state == TOMBSTONE:
        if first_tombstone is None: first_tombstone = i
    elif slots[i].key == key:
        slots[i] = OAEntry(key, value, OCCUPIED)   # update in place
        return new_map (same size)
raise Error("table full — should have resized earlier")
```

### `get(map, key) → value | None`

**Chaining:**
```
idx = _bucket_index(map, key)
for entry in map.buckets[idx].entries:
    if entry.key == key: return entry.value
return None
```

**Open addressing:**
```
idx = _bucket_index(map, key)
for probe in 0..capacity:
    i = (idx + probe) % capacity
    if slots[i].state == EMPTY: return None      # probe chain ended
    if slots[i].state == OCCUPIED and slots[i].key == key:
        return slots[i].value
    # TOMBSTONE or wrong key: continue probing
return None
```

### `delete(map, key) → HashMap`

**Chaining:**
```
idx = _bucket_index(map, key)
bucket = [e for e in map.buckets[idx].entries if e.key != key]
removed = len(bucket) < len(map.buckets[idx].entries)
return map with buckets[idx] = bucket, size -= 1 if removed
```

**Open addressing:**
```
idx = _bucket_index(map, key)
for probe in 0..capacity:
    i = (idx + probe) % capacity
    if slots[i].state == EMPTY: return map  (key not found)
    if slots[i].state == OCCUPIED and slots[i].key == key:
        slots[i].state = TOMBSTONE
        return map with size -= 1
```

### `_resize(map) → HashMap`

```
new_capacity = map.capacity * 2
new_map = new_map(capacity=new_capacity, strategy=map.strategy)
for each live (key, value) in map:
    new_map = set(new_map, key, value)
return new_map
```

### `keys(map) → list`

**Chaining:**
```
return [entry.key
        for bucket in map.buckets
        for entry in bucket.entries]
```

**Open addressing:**
```
return [slot.key for slot in map.slots if slot.state == OCCUPIED]
```

### `load_factor(map) → float`

```
return map.size / map.capacity
```

## Public API

```python
from typing import Any, Generic, TypeVar

K = TypeVar("K")
V = TypeVar("V")

class HashMap(Generic[K, V]):
    """
    Hash map with pluggable collision strategy and hash function.

    Functional interface: mutating operations return a new HashMap,
    leaving the original unchanged. This makes the API safe for
    immutable/persistent usage patterns.
    """
    pass

# Construction
def new_map(
    capacity:  int = 16,
    strategy:  str = "chaining",   # or "open_addressing"
    hash_fn:   str = "siphash"     # or "fnv1a", "murmur3"
) -> HashMap: ...

# Core operations
def set(map: HashMap[K, V], key: K, value: V) -> HashMap[K, V]: ...
def get(map: HashMap[K, V], key: K) -> V | None: ...
def delete(map: HashMap[K, V], key: K) -> HashMap[K, V]: ...
def has(map: HashMap[K, V], key: K) -> bool: ...

# Bulk access
def keys(map: HashMap[K, V]) -> list[K]: ...
def values(map: HashMap[K, V]) -> list[V]: ...
def entries(map: HashMap[K, V]) -> list[tuple[K, V]]: ...

# Introspection
def size(map: HashMap) -> int: ...
def load_factor(map: HashMap) -> float: ...
def capacity(map: HashMap) -> int: ...

# Utility
def from_entries(pairs: list[tuple[K, V]]) -> HashMap[K, V]: ...
def merge(m1: HashMap[K, V], m2: HashMap[K, V]) -> HashMap[K, V]: ...
```

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

```python
# Python — abstract base + two concrete strategies
from abc import ABC, abstractmethod

class HashMapStrategy(ABC):
    @abstractmethod
    def get(self, key) -> Any | None: ...
    @abstractmethod
    def set(self, key, value) -> None: ...
    @abstractmethod
    def delete(self, key) -> bool: ...

class ChainingStrategy(HashMapStrategy):
    def __init__(self, capacity, hash_fn):
        self._buckets  = [[] for _ in range(capacity)]
        self._hash_fn  = hash_fn
        self._capacity = capacity
    ...

class OpenAddressingStrategy(HashMapStrategy):
    EMPTY     = object()
    TOMBSTONE = object()
    def __init__(self, capacity, hash_fn):
        self._slots    = [self.EMPTY] * capacity
        self._hash_fn  = hash_fn
        self._capacity = capacity
    ...

class HashMap:
    def __init__(self, capacity=16, strategy="chaining", hash_fn="siphash"):
        self._strategy = (ChainingStrategy if strategy == "chaining"
                          else OpenAddressingStrategy)(capacity, hash_fn)
        self._size = 0
```

```typescript
// TypeScript — interface + two implementations
interface HashMapStrategy<K, V> {
  get(key: K): V | undefined;
  set(key: K, value: V): void;
  delete(key: K): boolean;
  entries(): Array<[K, V]>;
}

class ChainingStrategy<K, V> implements HashMapStrategy<K, V> { ... }
class OpenAddressingStrategy<K, V> implements HashMapStrategy<K, V> { ... }

class HashMap<K, V> {
  private strategy: HashMapStrategy<K, V>;
  constructor(opts: { capacity?: number; strategy?: "chaining" | "open" }) { ... }
  get(key: K): V | undefined { return this.strategy.get(key); }
  set(key: K, value: V): this { this.strategy.set(key, value); return this; }
}
```

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — enum for strategy discrimination
#[derive(Debug)]
pub enum CollisionStrategy {
    Chaining,
    OpenAddressing,
}

pub struct HashMap<K, V> {
    buckets:   Vec<Vec<(K, V)>>,    // chaining
    // OR
    slots:     Vec<Slot<K, V>>,     // open addressing
    strategy:  CollisionStrategy,
    size:      usize,
    capacity:  usize,
}

#[derive(Debug)]
pub enum Slot<K, V> {
    Empty,
    Tombstone,
    Occupied(K, V),
}

// Pure functions (impl block)
impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn new(capacity: usize, strategy: CollisionStrategy) -> Self { ... }
    pub fn get(&self, key: &K) -> Option<&V> { ... }
    pub fn set(self, key: K, value: V) -> Self { ... }
    pub fn delete(self, key: &K) -> Self { ... }
    pub fn load_factor(&self) -> f64 { self.size as f64 / self.capacity as f64 }
}
```

```go
// Go — struct with strategy enum
type Strategy int
const (
    Chaining Strategy = iota
    OpenAddressing
)

type Entry[K comparable, V any] struct {
    Key   K
    Value V
}

type HashMap[K comparable, V any] struct {
    buckets  [][]Entry[K, V]    // chaining
    slots    []slot[K, V]       // open addressing
    strategy Strategy
    size     int
    capacity int
    hashFn   func(any) uint64
}

func NewHashMap[K comparable, V any](capacity int, s Strategy) *HashMap[K, V] { ... }
func (m *HashMap[K, V]) Get(key K) (V, bool) { ... }
func (m *HashMap[K, V]) Set(key K, value V) { ... }
func (m *HashMap[K, V]) Delete(key K) bool { ... }
func (m *HashMap[K, V]) LoadFactor() float64 { return float64(m.size) / float64(m.capacity) }
```

```elixir
# Elixir — immutable maps as tagged structs
defmodule HashMap do
  defstruct [:buckets, :size, :capacity, :strategy]

  def new(capacity \\ 16, strategy \\ :chaining) do
    %HashMap{
      buckets:  List.duplicate([], capacity),
      size:     0,
      capacity: capacity,
      strategy: strategy
    }
  end

  def set(%HashMap{strategy: :chaining} = map, key, value) do
    idx = bucket_index(map, key)
    bucket = Enum.at(map.buckets, idx)
    {new_bucket, replaced} = update_bucket(bucket, key, value)
    new_size = if replaced, do: map.size, else: map.size + 1
    new_map = %{map | buckets: List.replace_at(map.buckets, idx, new_bucket),
                      size: new_size}
    maybe_resize(new_map)
  end

  defp bucket_index(map, key) do
    :erlang.phash2(key, map.capacity)   # Erlang's built-in hash
  end

  defp maybe_resize(%HashMap{size: n, capacity: cap} = map) when n / cap > 1.0 do
    resize(map)
  end
  defp maybe_resize(map), do: map
end
```

## Test Strategy

### Unit tests

```
# Basic set/get
m = new_map()
m = set(m, "a", 1)
get(m, "a")   → 1
get(m, "b")   → None
has(m, "a")   → True
has(m, "b")   → False
size(m)       → 1

# Overwrite
m = set(m, "a", 99)
get(m, "a")   → 99
size(m)       → 1   (not 2)

# Multiple keys
m = set(set(set(new_map(), "x", 10), "y", 20), "z", 30)
get(m, "x") → 10;  get(m, "y") → 20;  get(m, "z") → 30
size(m) → 3
keys(m) contains "x", "y", "z"

# Deletion
m = delete(m, "y")
get(m, "y") → None
size(m) → 2
has(m, "y") → False

# Delete nonexistent key → no error, map unchanged
m2 = delete(m, "nonexistent")
size(m2) == size(m)

# Collision handling (chaining)
# Force two keys to the same bucket by using capacity=1
m = new_map(capacity=1, strategy="chaining")
m = set(set(m, "cat", 1), "car", 2)
get(m, "cat") → 1
get(m, "car") → 2

# Collision handling (open addressing)
m = new_map(capacity=4, strategy="open_addressing")
# Insert 3 keys that all hash to slot 0 (use keys known to collide mod 4)
# Verify all three are retrievable

# Resize triggered
m = new_map(capacity=4, strategy="chaining")
m = set(set(set(set(set(m, "a",1),"b",2),"c",3),"d",4),"e",5)
# After 5th insert, load_factor was 5/4 = 1.25 → resize to 8
capacity(m) → 8
size(m) → 5
# All keys still accessible
get(m, "a") → 1;  get(m, "e") → 5

# Tombstone / open addressing deletion
m = new_map(capacity=8, strategy="open_addressing")
m = set(set(m, "cat", 1), "car", 2)   # car probed past cat
m = delete(m, "cat")
get(m, "car") → 2   # must still work despite cat's slot being tombstone

# entries / keys / values
m = from_entries([("x", 1), ("y", 2), ("z", 3)])
set(entries(m)) == {("x",1), ("y",2), ("z",3)}

# merge
m1 = from_entries([("a", 1), ("b", 2)])
m2 = from_entries([("b", 99), ("c", 3)])
m3 = merge(m1, m2)
get(m3, "a") → 1;  get(m3, "b") → 99;  get(m3, "c") → 3
```

### Property-based tests

- For any sequence of set/delete operations, the result matches a reference
  Python `dict` performing the same operations.
- After resize, all existing keys are still retrievable.
- `size(m)` always equals `len(keys(m))`.
- `load_factor(m)` is always <= resize threshold after any operation.
- For open addressing: no key is ever "lost" by a tombstone (test by
  inserting colliding keys then deleting the first one and querying the rest).

### Performance tests

- 100,000 insertions into a chaining map complete in < 1 second.
- 100,000 insertions into an open-addressing map complete in < 500 ms.
- 100,000 lookups (all hits) in a 100,000-element map complete in < 100 ms.
- Average probe length in open-addressing map at α=0.5 is ≤ 2.

## Future Extensions

- **DT19: Hash set** — a hash map where we only store keys (no values). All
  the same logic; just drop the value field.
- **Ordered hash map** — maintains insertion order. Python 3.7+ dicts do this
  by keeping a separate insertion-order linked list.
- **LRU cache** — combines a hash map (O(1) lookup) with a doubly linked list
  (O(1) eviction of least-recently-used). The classic system design interview
  data structure.
- **Concurrent hash map** — lock striping or lock-free compare-and-swap for
  thread safety. Java's `ConcurrentHashMap` uses 16 lock stripes by default.
- **Perfect hashing** — when the key set is known in advance, you can
  construct a hash function with zero collisions. Used in compiler keyword
  tables, routing tables.
- **Cuckoo hashing** — each key has two possible bucket locations; insertion
  can "kick out" existing keys into their alternate slot. Worst-case O(1)
  lookup. Used in network packet classifiers.
