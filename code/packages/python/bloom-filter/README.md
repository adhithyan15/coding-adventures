# bloom-filter — DT22

Space-efficient probabilistic set membership filter. Part of the
[coding-adventures](https://github.com/adhithya/coding-adventures) series.

## What it does

A Bloom filter answers "Have I seen this element before?" with two possible
answers:

- **"Definitely NOT in set"** — zero false negatives. If the filter says NO,
  the element was guaranteed to never have been added.
- **"Probably in set"** — small, tunable probability of false positives. The
  filter says YES, but occasionally the element was never added.

This asymmetry makes Bloom filters ideal as pre-flight checks before expensive
operations: a disk read, a network request, a cache lookup. If the filter says
NO, skip the operation. If it says YES, do it (and occasionally handle a false
positive, which is acceptable).

## Where it fits

```
DT17: hash-functions      ← core primitive (fnv1a_32, djb2 for double hashing)
DT19: hash-set            ← exact membership (O(n) space)
DT21: hyperloglog         ← approximate counting
DT22: bloom-filter        ← [HERE] approximate membership (O(m) bits)

DT25: mini-redis          ← could use bloom filter as a pre-check layer
```

**Depends on:** `coding-adventures-hash-functions` (DT17).

## Installation

```bash
pip install coding-adventures-bloom-filter
```

## Usage

```python
from bloom_filter import BloomFilter

# Create a filter for ~1000 elements with a 1% false positive rate.
bf = BloomFilter(expected_items=1000, false_positive_rate=0.01)

# Add elements.
bf.add("alice")
bf.add("bob")

# Check membership using contains() or the `in` operator.
"alice" in bf        # True  — definitely was added
"carol" in bf        # False — definitely was NOT added (no FN)
"dave" in bf         # False — or occasionally True (false positive, ~1%)

# Inspect the filter state.
bf.bit_count         # total bits m
bf.hash_count        # number of hash functions k
bf.fill_ratio        # fraction of bits currently set
bf.estimated_false_positive_rate  # estimated current FPR

# Create a filter with explicit parameters (bypass auto-sizing).
bf2 = BloomFilter.from_params(bit_count=10_000, hash_count=7)
```

## Algorithm

### Bit array + double hashing

The filter maintains a bit array of `m` bits, all initially 0. Adding an
element sets `k` bits; checking an element verifies those same bits.

Double hashing generates `k` bit positions from two base hashes:

```
g_i(x) = (fnv1a_32(x) + i * djb2(x)) mod m   for i = 0, ..., k-1
```

### Optimal parameters

Given expected items `n` and target false positive rate `p`:

```
m = ceil(-n * ln(p) / ln(2)^2)   ← bit array size
k = max(1, round((m / n) * ln(2)))  ← hash function count
```

### Memory comparison (n = 1,000,000 elements)

| FPR  | Bits/elem | Memory     |
|------|-----------|------------|
| 10%  | 4.79      | 585 KB     |
| 1%   | 9.58      | 1.14 MB    |
| 0.1% | 14.38     | 1.72 MB    |
| vs exact hash set: ~40 MB (35× larger) |

## API Reference

```python
class BloomFilter:
    def __init__(self, expected_items=1000, false_positive_rate=0.01): ...
    def add(self, element) -> None: ...
    def contains(self, element) -> bool: ...
    def __contains__(self, element) -> bool: ...  # `in` operator

    @property
    def bit_count(self) -> int: ...
    @property
    def hash_count(self) -> int: ...
    @property
    def bits_set(self) -> int: ...
    @property
    def fill_ratio(self) -> float: ...
    @property
    def estimated_false_positive_rate(self) -> float: ...

    def is_over_capacity(self) -> bool: ...
    def size_bytes(self) -> int: ...

    @staticmethod
    def optimal_m(n, p) -> int: ...
    @staticmethod
    def optimal_k(m, n) -> int: ...
    @staticmethod
    def capacity_for_memory(memory_bytes, p) -> int: ...

    @classmethod
    def from_params(cls, bit_count, hash_count) -> BloomFilter: ...
```

## Development

```bash
uv venv .venv
uv pip install -e ../hash-functions
uv pip install -e .[dev]
uv run python -m pytest tests/ -v
```

## Real-world deployments

- **LevelDB / RocksDB / Cassandra** — avoid disk seeks for missing SSTable keys
- **Chrome Safe Browsing** — local Bloom check before network call to Google
- **Akamai CDN** — cache URLs seen twice or more (avoid one-hit pollution)
- **Bitcoin SPV** — filter transactions without revealing watched addresses
