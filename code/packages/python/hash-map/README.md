# coding-adventures-hash-map

**DT18** — Hash map with separate chaining and open addressing.

## What it is

A generic `HashMap[K, V]` built from scratch, implementing the two most
important collision-resolution strategies:

| Strategy | Collision handling | Resize threshold |
|---|---|---|
| `"chaining"` | Bucket lists | load factor > 1.0 |
| `"open_addressing"` | Linear probing + tombstones | load factor > 0.75 |

Both strategies support pluggable hash functions from
[coding-adventures-hash-functions](../hash-functions): `"fnv1a"` (default),
`"murmur3"`, and `"djb2"`.

## Layer position

```
DT17: hash-functions  ← dependency (fnv1a_32, murmur3_32, djb2)
  └── DT18: hash-map  ← YOU ARE HERE
        └── DT19: hash-set
```

## Installation

```bash
pip install coding-adventures-hash-map
```

## Quick start

```python
from hash_map import HashMap, from_entries, merge

# Default: chaining strategy, fnv1a hash function, capacity 16
m = HashMap()
m.set("hello", 42)
m.get("hello")   # 42
m.has("hello")   # True
m.delete("hello")
m.size()         # 0

# Open addressing
m2 = HashMap(capacity=8, strategy="open_addressing", hash_fn="murmur3")
m2.set("a", 1)
m2.set("b", 2)
len(m2)          # 2

# Bulk construction
m3 = from_entries([("x", 10), ("y", 20), ("z", 30)])
m3.keys()        # ["x", "y", "z"] (order may vary)
m3.entries()     # [("x", 10), ("y", 20), ("z", 30)] (order may vary)

# Merge (m2 values override m1 on key conflict)
m4 = from_entries([("a", 1), ("b", 2)])
m5 = from_entries([("b", 99), ("c", 3)])
m6 = merge(m4, m5)
m6.get("b")      # 99

# Python protocols
for key in m3:
    print(key, m3.get(key))

"x" in m3        # True
```

## Public API

```python
class HashMap(Generic[K, V]):
    def __init__(self, capacity: int = 16, strategy: str = "chaining", hash_fn: str = "fnv1a") -> None
    def set(self, key: K, value: V) -> None
    def get(self, key: K) -> V | None
    def delete(self, key: K) -> bool         # True if deleted
    def has(self, key: K) -> bool
    def keys(self) -> list[K]
    def values(self) -> list[V]
    def entries(self) -> list[tuple[K, V]]
    def size(self) -> int
    def load_factor(self) -> float
    def capacity(self) -> int
    def __len__(self) -> int
    def __contains__(self, key: object) -> bool
    def __iter__(self) -> Iterator[K]
    def __repr__(self) -> str

def from_entries(pairs: list[tuple[K, V]], strategy: str = "chaining", hash_fn: str = "fnv1a") -> HashMap[K, V]
def merge(m1: HashMap[K, V], m2: HashMap[K, V]) -> HashMap[K, V]
```

## How it works

### Separate chaining

Each bucket in the array holds a Python list of `(key, value)` pairs.
Collisions are handled by appending to the list:

```
Capacity=4, inserting "cat" (hash%4=3) and "car" (hash%4=3):

slot 0: []
slot 1: []
slot 2: []
slot 3: [("cat", 1), ("car", 2)]   ← both in same bucket
```

Lookup scans the bucket list linearly — O(1) average when the load factor
is low (chains are length 0 or 1 most of the time).

### Open addressing (linear probing + tombstones)

All entries live in a single flat array. On collision, the next slot is
tried (wrapping around):

```
Capacity=4, inserting "cat" (hash%4=3) then "car" (hash%4=3):

slot 0: EMPTY
slot 1: EMPTY
slot 2: EMPTY
slot 3: ("cat", 1)

Insert "car": slot 3 occupied → try slot 0 (wrap)
slot 0: ("car", 2)
slot 3: ("cat", 1)
```

Deletion places a `TOMBSTONE` instead of clearing the slot, so probe
chains for other keys remain intact.

### Automatic resizing

When the load factor exceeds the threshold, the capacity doubles and all
entries are re-hashed into the new array. This is O(n) but happens
infrequently — amortised O(1) per insertion.

## Running tests

```bash
cd code/packages/python/hash-map
uv venv .venv --no-project
uv pip install --python .venv -e ../hash-functions
uv pip install --python .venv -e .[dev]
uv run --no-project python -m pytest tests/ -v
```
