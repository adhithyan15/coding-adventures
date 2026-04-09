# DT22 — Bloom Filter

## Overview

A **Bloom filter** answers the question "Have I seen this element before?"
with two possible answers:

- **"Definitely NO"** — The element is *guaranteed* to not be in the set.
  Zero false negatives. If the filter says NO, trust it completely.

- **"Probably YES"** — The element *might* be in the set. There is a small,
  controllable probability that this is a false positive (the filter says YES
  but the element was never actually added).

This asymmetry makes Bloom filters extremely useful for "pre-flight checks"
that avoid expensive operations — a database disk read, a network request, a
cache fetch. If the Bloom filter says NO, skip the expensive operation. If it
says YES, do the expensive operation to confirm (and occasionally find a false
positive, which is fine).

```
Bloom filter vs Hash Set:

Question: "Is 'foobar' in the set?"

Hash Set:
  ┌──────────────────────────────────────────────┐
  │ "alice", "bob", "carol", ..., 1M entries     │
  │                ~32 MB                         │
  └──────────────────────────────────────────────┘
  Answer: YES or NO (exact, never wrong)
  Cost: 32 MB RAM for 1M strings

Bloom Filter (1% false positive rate):
  ┌──────────────────────────────────────────────┐
  │ 0110100011010001101001001010100110100010...   │
  │              ~1.2 MB bit array                │
  └──────────────────────────────────────────────┘
  Answer: "Definitely NOT in set" or "Probably in set"
  Cost: 1.2 MB for 1M strings (27× smaller!)
  False positive rate: 1% (tunable)
```

## Layer Position

```
DT17: hash-functions      ← core primitive (need k independent hash functions)
DT19: hash-set            ← exact membership (O(n) space)
DT21: hyperloglog         ← approximate counting (not membership)
DT22: bloom-filter        ← [YOU ARE HERE] approximate membership (O(1) space)

DT25: mini-redis          ← could use bloom filter as a pre-check layer
```

**Depends on:** DT17 (hash functions — need k independent, fast functions).
**Contrasts with:** DT19 (exact, larger) and DT21 (counts, doesn't answer membership).
**Used by:** Databases (avoid reading disk for missing keys), CDN caches
(has this URL been cached?), Chrome Safe Browsing (is this URL malicious?),
spell checkers, network routers, distributed systems (Cassandra, HBase,
LevelDB, RocksDB — every one of them uses Bloom filters for SSTable lookups).

## Concepts

### The Problem: Expensive Existence Checks

A classic database problem: before reading a data block from disk (slow: ~10ms),
check if the key *might* exist. If it definitely doesn't exist, skip the disk read.

```
Without Bloom filter:
  client: "Get key 'missing_key'"
  database: read disk block... read disk block... read disk block...
  → 3 disk reads to confirm key doesn't exist
  → 30ms wasted

With Bloom filter:
  client: "Get key 'missing_key'"
  database: check bloom filter → "Definitely NOT in this SSTable"
  → 0 disk reads, 0ms
  → Bloom filter was right 99% of the time (false positive rate = 1%)
```

LevelDB and RocksDB use a 10-bit Bloom filter per key. For a database with
100 million keys, the Bloom filter for one SSTable file uses ~120 MB total —
spread across thousands of files, each filtering millions of unnecessary reads.

### How It Works: Bit Array + Multiple Hash Functions

A Bloom filter is a bit array of m bits, initially all set to 0.
To add an element, we set k bits to 1. To check membership, we verify
those same k bits are still 1.

```
Empty Bloom filter (m=16 bits, k=3 hash functions):
Index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
Bits:  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0

Add "alice":
  h1("alice") mod 16 = 3   → set bit 3
  h2("alice") mod 16 = 7   → set bit 7
  h3("alice") mod 16 = 11  → set bit 11

Index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
Bits:  0  0  0  1  0  0  0  1  0  0  0  1  0  0  0  0
                ↑              ↑              ↑
          set by h1      set by h2      set by h3

Add "bob":
  h1("bob") mod 16 = 1   → set bit 1
  h2("bob") mod 16 = 5   → set bit 5
  h3("bob") mod 16 = 11  → already 1 (collision, that's fine)

Index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
Bits:  0  1  0  1  0  1  0  1  0  0  0  1  0  0  0  0
```

Now checking membership:

```
contains("alice")?
  h1("alice") mod 16 = 3  → bit 3 is 1 ✓
  h2("alice") mod 16 = 7  → bit 7 is 1 ✓
  h3("alice") mod 16 = 11 → bit 11 is 1 ✓
  All k bits are 1 → "Probably YES" → CORRECT

contains("carol")?  (carol was never added)
  h1("carol") mod 16 = 2  → bit 2 is 0 ✗
  → At least one bit is 0 → "Definitely NO" → CORRECT

contains("dave")?   (dave was never added — but what if we get unlucky?)
  h1("dave") mod 16 = 1   → bit 1 is 1 (was set by "bob") ✓
  h2("dave") mod 16 = 5   → bit 5 is 1 (was set by "bob") ✓
  h3("dave") mod 16 = 11  → bit 11 is 1 (set by both "alice" and "bob") ✓
  All k bits are 1 → "Probably YES" → FALSE POSITIVE! Dave was never added.
```

This false positive is why the filter says "Probably" not "Definitely".
The probability of this happening is controlled by m, k, and n.

### Why You CAN'T Delete from a Standard Bloom Filter

Once a bit is set to 1, you can't clear it — another element might have set
that same bit. If you clear it, you break membership for that other element.

```
Before delete "bob":
  Index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
  Bits:  0  1  0  1  0  1  0  1  0  0  0  1  0  0  0  0

"Delete" bob: clear bits 1, 5 (h1, h2 for bob). Bit 11 is shared with alice.

  Index: 0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
  Bits:  0  0  0  1  0  0  0  1  0  0  0  1  0  0  0  0
                                               ↑
                               Still set because alice uses it.
                               (Fine for bit 11.)

Now check "alice":
  h1("alice") mod 16 = 3  → bit 3 is 1 ✓
  h2("alice") mod 16 = 7  → bit 7 is 1 ✓
  h3("alice") mod 16 = 11 → bit 11 is 1 ✓
  Still "Probably YES" — alice is fine!

But what if "alice" had used bit 1 or bit 5? Deleting bob would incorrectly
mark alice as absent. This is why deletion is not supported.
```

The fix is the **Counting Bloom Filter** (see Future Extensions), which
replaces each bit with a small counter.

### The Math: False Positive Probability

Given:
- m = total number of bits
- k = number of hash functions
- n = number of elements inserted

The probability that any specific bit is still 0 after inserting n elements:
  p(bit still 0) = (1 - 1/m)^(kn) ≈ e^(-kn/m)

The probability that all k bits for a new element are 1 (false positive):
  p(false positive) ≈ (1 - e^(-kn/m))^k

Let's call the fill ratio: load = kn/m. Then:

```
p ≈ (1 - e^(-load))^k

For k=3, n=1000, m=14378 (optimal m for 1%):
  load = 3 × 1000 / 14378 ≈ 0.2087
  p ≈ (1 - e^(-0.2087))^3 ≈ (1 - 0.812)^3 ≈ (0.188)^3 ≈ 0.0067 ≈ 0.67%
  Close to our target of 1%.
```

### Optimal Number of Hash Functions

For a given m and n, what k minimizes the false positive rate?

Taking the derivative of the false positive formula with respect to k and
setting it to zero:

```
k_optimal = (m/n) × ln(2) ≈ 0.693 × (m/n)

Intuition: we want each bit to have about a 50% chance of being set.
  - Too few hash functions (k small): few bits set, but large p because
    any non-element has all k bits set by coincidence easily.
  - Too many hash functions (k large): many bits set, most elements share
    bits with non-elements.
  - Just right: k ≈ 0.693 × (m/n) minimizes the product.
```

### Optimal Bit Array Size

For a desired false positive rate p and n elements:

```
m_optimal = -n × ln(p) / (ln(2))²
           = -n × ln(p) / 0.4805

Memory table for n=1,000,000 elements:

False Positive Rate   Bits per element   Total bits (m)   Memory
-------------------   ----------------   --------------   -------
10%                   4.79               4,792,536        585 KB
1%                    9.58               9,585,058        1.14 MB
0.1%                  14.38              14,377,588       1.72 MB
0.01%                 19.17              19,170,114       2.29 MB
0.001%                23.96              23,962,641       2.86 MB

Compare: exact hash set for 1M strings (avg 20 chars):
  20 bytes/string × 1M = 20 MB + hash map overhead ≈ 40 MB
  Bloom filter at 1%: 1.14 MB — 35× smaller!
```

### Multiple Hash Functions from One: Double Hashing

Computing k truly independent hash functions is expensive. Instead, use
the **double hashing** trick:

```
Given two independent hashes h1 and h2, define:
  g_i(x) = (h1(x) + i × h2(x)) mod m    for i = 0, 1, ..., k-1

This gives k distinct positions using only two hash computations.
The two hash functions come from DT17 (e.g., MurmurHash3 returns two
128-bit values, or use MurmurHash for h1 and FNV-1a for h2).
```

Theoretically, these are not perfectly independent, but in practice they
perform equivalently to truly independent hash functions for Bloom filter
purposes. All major implementations use this trick.

```python
def get_k_positions(element, m, k):
    h1 = murmur_hash(element, seed=0)
    h2 = murmur_hash(element, seed=h1)   # or use a different hash fn
    positions = []
    for i in range(k):
        pos = (h1 + i * h2) % m
        positions.append(pos)
    return positions
```

### Real-World Use Cases

**Google Chrome Safe Browsing:**
Chrome maintains a Bloom filter of ~650,000 known malicious URLs. Before
making a network call to Google's servers to check a URL, Chrome checks
the local Bloom filter. If NO, skip the expensive network call. If YES,
verify with the server (handling false positives gracefully).

**Apache Cassandra / HBase / LevelDB / RocksDB:**
These databases use log-structured merge trees (LSM trees). Data is written
to multiple sorted files (SSTables). When reading a key, the database must
check each SSTable. Without Bloom filters, every SSTable requires a disk
seek. With Bloom filters, only SSTables that might contain the key are
checked. For missing keys (very common in production workloads), Bloom
filters eliminate virtually all disk reads.

**Akamai CDN:**
Uses Bloom filters to decide whether to cache a URL. URLs seen only once
(one-hit wonders) are not cached. Only URLs seen a second time (confirmed
in the Bloom filter) get cached. This prevents cache pollution from one-time
requests.

**Bitcoin:**
SPV (Simplified Payment Verification) clients download Bloom filters from
full nodes to filter which transactions to send back. This reduces bandwidth
while preserving privacy (the full node doesn't learn exactly which addresses
the SPV client cares about).

**Spell Checkers:**
Dictionary words go into the Bloom filter. Any word not in the filter is
definitely misspelled. Words in the filter are probably correct (false
positives are acceptable — they just mean we miss some misspellings).

## Representation

```
BloomFilter {
    m: int           # total number of bits
    k: int           # number of hash functions
    n: int           # number of elements inserted so far
    bits: BitArray   # m-bit array, all initially 0
}

BitArray: a compact array of bits, stored as an array of bytes.
  m=1,000,000 bits → 125,000 bytes = 125 KB

# For n=1,000,000 expected elements and p=0.01 (1% FPR):
#   m = ceil(-1M × ln(0.01) / ln(2)²) = 9,585,059 bits ≈ 1.14 MB
#   k = round(0.693 × m / n) = round(0.693 × 9.585) = 7
```

### BitArray Storage

```
Individual bit access:
  bit_index i → byte_index = i // 8, bit_offset = i % 8

  Set bit i:
    bytes[i // 8] |= (1 << (i % 8))

  Clear bit i:
    bytes[i // 8] &= ~(1 << (i % 8))

  Get bit i:
    (bytes[i // 8] >> (i % 8)) & 1

Example: m=16 bits stored in 2 bytes
  Byte 0: bits 0-7
  Byte 1: bits 8-15

  Set bit 3:  byte 0 |= 0b00001000 → byte 0 = 0b00001000
  Set bit 11: byte 1 |= 0b00001000 → byte 1 = 0b00001000

  ┌─────────────────────────────────┐
  │ byte 0: 0 0 0 0 1 0 0 0        │  bit 3 is set
  │ byte 1: 0 0 0 0 1 0 0 0        │  bit 11 is set
  └─────────────────────────────────┘
  bits:  0 1 2 3 4 5 6 7 | 8 9 10 11 12 13 14 15
```

## Algorithms (Pure Functions)

### optimal_m(n, p) → int

```
optimal_m(n, p):
    """Optimal bit array size for n elements and false positive rate p."""
    import math
    return math.ceil(-n * math.log(p) / (math.log(2) ** 2))

Examples:
  optimal_m(1_000_000, 0.01)   = 9,585,059  (1% FPR)
  optimal_m(1_000_000, 0.001)  = 14,377,588 (0.1% FPR)
  optimal_m(10_000_000, 0.01)  = 95,850,584 (10M elements, 1% FPR)
```

### optimal_k(m, n) → int

```
optimal_k(m, n):
    """Optimal number of hash functions."""
    import math
    return max(1, round((m / n) * math.log(2)))

Examples:
  optimal_k(9_585_059, 1_000_000)  = 7
  optimal_k(14_377_588, 1_000_000) = 10
```

### current_fpr(bf) → float

```
current_fpr(bf):
    """Estimated false positive rate given how full the filter currently is."""
    import math
    # Fill ratio: expected fraction of bits that are 1
    fill = 1 - math.e ** (-bf.k * bf.n / bf.m)
    return fill ** bf.k

# This rises as more elements are added:
# After inserting 500K of expected 1M:
#   fill ratio ≈ 0.5, fpr ≈ 0.5^7 ≈ 0.78%
# After inserting 1M (at capacity):
#   fill ratio ≈ 0.693, fpr ≈ 1%
# After inserting 2M (over capacity!):
#   fill ratio → 1, fpr → 100%
```

### add(bf, element) → BloomFilter

```
add(bf, element):
    positions = double_hash_positions(element, bf.m, bf.k)
    new_bits = copy(bf.bits)
    for pos in positions:
        set_bit(new_bits, pos)
    return BloomFilter(m=bf.m, k=bf.k, n=bf.n+1, bits=new_bits)
```

### contains(bf, element) → bool

```
contains(bf, element):
    positions = double_hash_positions(element, bf.m, bf.k)
    for pos in positions:
        if not get_bit(bf.bits, pos):
            return False   # definitely not in set
    return True            # probably in set
```

### double_hash_positions(element, m, k) → list[int]

```
double_hash_positions(element, m, k):
    h1 = murmur3_32(element, seed=0)
    h2 = murmur3_32(element, seed=h1)
    return [(h1 + i * h2) % m for i in range(k)]
```

## Public API

```python
class BloomFilter:
    """
    A space-efficient probabilistic data structure for set membership.

    No false negatives: if contains() returns False, the element is
    definitely not in the set.

    Possible false positives: if contains() returns True, the element
    is probably in the set, but there is a small probability it is not.

    The false positive rate is controlled by expected_elements and
    false_positive_rate at construction time.
    """

    def __init__(
        self,
        expected_elements: int,
        false_positive_rate: float = 0.01
    ) -> "BloomFilter":
        """
        Create a Bloom filter optimized for the given parameters.
        Automatically computes optimal m and k.

        expected_elements: how many distinct elements you plan to add
        false_positive_rate: fraction between 0 and 1 (e.g., 0.01 = 1%)
        """

    def add(self, element) -> "BloomFilter":
        """
        Record that element has been seen.
        Returns a new BloomFilter (functional style).
        O(k) hash computations, O(k) bit operations.
        """

    def contains(self, element) -> bool:
        """
        Check if element might be in the set.
        Returns False → element is DEFINITELY not in the set.
        Returns True  → element is PROBABLY in the set.
        O(k) operations.
        """

    def current_fpr(self) -> float:
        """
        Estimated false positive rate given the number of elements
        added so far. Rises as the filter fills up.
        """

    def is_over_capacity(self) -> bool:
        """
        True if more elements than expected_elements have been added.
        When over capacity, false positive rate exceeds the target.
        """

    def size_bytes(self) -> int:
        """Memory usage in bytes (m / 8, rounded up)."""

    @staticmethod
    def optimal_m(n: int, p: float) -> int:
        """Optimal bit array size for n elements and false positive rate p."""

    @staticmethod
    def optimal_k(m: int, n: int) -> int:
        """Optimal number of hash functions for m bits and n elements."""

    @staticmethod
    def capacity_for_memory(memory_bytes: int, p: float) -> int:
        """
        How many elements can we store in memory_bytes bytes
        at false positive rate p?
        Inverse of optimal_m.
        """
```

## Composition Model

Bloom filter composes on DT17 (hash functions) and wraps a plain bit array.
It is the first DT layer that doesn't build on a previous DT container.

### Python / Ruby / TypeScript — Class with bytearray

```python
# Python: use bytearray for mutable bit storage
import math

class BloomFilter:
    def __init__(self, expected_elements: int, fpr: float = 0.01):
        self.n_expected = expected_elements
        self.fpr_target = fpr
        self.m = self.optimal_m(expected_elements, fpr)
        self.k = self.optimal_k(self.m, expected_elements)
        self.n = 0
        self._bytes = bytearray(math.ceil(self.m / 8))

    def _positions(self, element):
        from mmh3 import hash  as mmh3_hash   # MurmurHash3 from DT17
        h1 = mmh3_hash(str(element), seed=0, signed=False)
        h2 = mmh3_hash(str(element), seed=h1, signed=False)
        return [(h1 + i * h2) % self.m for i in range(self.k)]

    def add(self, element) -> "BloomFilter":
        new_bf = BloomFilter.__new__(BloomFilter)
        new_bf.__dict__.update(self.__dict__)
        new_bf._bytes = bytearray(self._bytes)
        for pos in self._positions(element):
            new_bf._bytes[pos // 8] |= (1 << (pos % 8))
        new_bf.n = self.n + 1
        return new_bf

    def contains(self, element) -> bool:
        for pos in self._positions(element):
            if not (self._bytes[pos // 8] >> (pos % 8)) & 1:
                return False
        return True
```

### Rust — Bit Vector Crate

```rust
// Rust: use bitvec crate for efficient bit storage
use bitvec::prelude::*;
use murmur3::murmur3_32;

pub struct BloomFilter {
    bits: BitVec<u8, Lsb0>,
    m: usize,
    k: usize,
    n: usize,
}

impl BloomFilter {
    fn positions(&self, element: &[u8]) -> Vec<usize> {
        let h1 = murmur3_32(&mut std::io::Cursor::new(element), 0).unwrap() as usize;
        let h2 = murmur3_32(&mut std::io::Cursor::new(element), h1 as u32).unwrap() as usize;
        (0..self.k).map(|i| (h1 + i * h2) % self.m).collect()
    }

    pub fn add(&self, element: &[u8]) -> Self {
        let mut new_bits = self.bits.clone();
        for pos in self.positions(element) {
            new_bits.set(pos, true);
        }
        BloomFilter { bits: new_bits, n: self.n + 1, ..*self }
    }

    pub fn contains(&self, element: &[u8]) -> bool {
        self.positions(element).iter().all(|&pos| self.bits[pos])
    }
}
```

### Go — Functional with uint64 Backing

```go
type BloomFilter struct {
    bits []uint64   // m bits packed into 64-bit words
    m    int
    k    int
    n    int
}

func (bf BloomFilter) positions(element []byte) []int {
    h1 := murmur3.Sum32(element)
    h2 := murmur3.SeedSum32(h1, element)
    positions := make([]int, bf.k)
    for i := range positions {
        positions[i] = int((uint64(h1) + uint64(i)*uint64(h2)) % uint64(bf.m))
    }
    return positions
}

func (bf BloomFilter) Add(element []byte) BloomFilter {
    newBits := make([]uint64, len(bf.bits))
    copy(newBits, bf.bits)
    for _, pos := range bf.positions(element) {
        newBits[pos/64] |= 1 << (pos % 64)
    }
    return BloomFilter{bits: newBits, m: bf.m, k: bf.k, n: bf.n + 1}
}

func (bf BloomFilter) Contains(element []byte) bool {
    for _, pos := range bf.positions(element) {
        if (bf.bits[pos/64] >> (pos % 64)) & 1 == 0 {
            return false
        }
    }
    return true
}
```

### Elixir — Binary for Bit Array

```elixir
# Elixir: use Erlang's :binary module and bitstring patterns
defmodule BloomFilter do
  defstruct [:m, :k, :n, :bits]

  def new(expected_n, fpr \\ 0.01) do
    m = optimal_m(expected_n, fpr)
    k = optimal_k(m, expected_n)
    # Erlang bitstring: m bits, all 0
    bits = <<0::size(m)>>
    %BloomFilter{m: m, k: k, n: 0, bits: bits}
  end

  def add(%BloomFilter{} = bf, element) do
    positions = double_hash_positions(element, bf.m, bf.k)
    new_bits = Enum.reduce(positions, bf.bits, &set_bit(&2, &1))
    %{bf | bits: new_bits, n: bf.n + 1}
  end
end
```

## Test Strategy

### No False Negatives

```python
def test_no_false_negatives():
    """Elements that were added must always be found."""
    bf = BloomFilter(expected_elements=10_000, false_positive_rate=0.01)
    added = set()

    for i in range(10_000):
        element = f"element_{i}"
        bf = bf.add(element)
        added.add(element)

    # Every added element must return True
    for element in added:
        assert bf.contains(element), f"False negative for {element}!"
```

### False Positive Rate

```python
def test_false_positive_rate():
    """False positive rate should be near the configured target."""
    target_fpr = 0.01
    bf = BloomFilter(expected_elements=100_000, false_positive_rate=target_fpr)

    for i in range(100_000):
        bf = bf.add(f"real_element_{i}")

    # Test with elements that were definitely never added
    false_positives = 0
    trials = 100_000
    for i in range(trials):
        if bf.contains(f"fake_element_{i}"):   # these were never added
            false_positives += 1

    actual_fpr = false_positives / trials
    # Should be within 2× of target (probabilistic — could rarely fail)
    assert actual_fpr < 2 * target_fpr, \
        f"FPR too high: {actual_fpr:.3%} vs target {target_fpr:.1%}"
    assert actual_fpr > 0, "Zero false positives is suspicious for large n"
    print(f"Actual FPR: {actual_fpr:.3%} (target: {target_fpr:.1%})")
```

### Optimal m and k

```python
def test_optimal_params():
    # For 1M elements, 1% FPR:
    m = BloomFilter.optimal_m(1_000_000, 0.01)
    k = BloomFilter.optimal_k(m, 1_000_000)

    assert 9_500_000 < m < 9_700_000   # ~9.585 million bits
    assert k == 7                        # optimal k

    # For 1M elements, 0.1% FPR:
    m2 = BloomFilter.optimal_m(1_000_000, 0.001)
    assert m2 > m   # more bits needed for lower FPR

def test_memory_size():
    bf = BloomFilter(expected_elements=1_000_000, false_positive_rate=0.01)
    # Should be approximately 1.14 MB
    assert 1_100_000 < bf.size_bytes() < 1_200_000
```

### Over-Capacity Behavior

```python
def test_over_capacity_increases_fpr():
    """Adding more elements than expected raises false positive rate."""
    n = 1_000
    bf = BloomFilter(expected_elements=n, false_positive_rate=0.01)

    # Add 2× the expected elements
    for i in range(2 * n):
        bf = bf.add(f"element_{i}")

    assert bf.is_over_capacity()
    assert bf.current_fpr() > 0.01   # FPR should exceed target
```

### Determinism

```python
def test_deterministic():
    """Same elements in same order → same bit array."""
    bf1 = BloomFilter(expected_elements=1000, false_positive_rate=0.01)
    bf2 = BloomFilter(expected_elements=1000, false_positive_rate=0.01)

    for i in range(100):
        bf1 = bf1.add(f"item_{i}")
        bf2 = bf2.add(f"item_{i}")

    assert bf1._bytes == bf2._bytes
```

## Future Extensions

**Counting Bloom Filter:** Replace each bit with a 4-bit counter. Now we
can support deletion: add increments the counter, delete decrements it.
A position is "set" if its counter > 0. The cost: 4× more memory. The
benefit: safe deletion. Used in network routers for IP flow tracking.

**Scalable Bloom Filter (SBF):** When the filter approaches capacity (FPR
rising above threshold), add a new Bloom filter with a larger bit array.
Union membership across all layers. Supports unbounded growth while
maintaining bounded FPR. An element is in the set if it's in ANY layer.

**Cuckoo Filter:** A more modern alternative to Bloom filters that:
- Supports deletion natively (without extra memory)
- Achieves better space efficiency for FPR < 3%
- Has slightly higher lookup performance
Uses a cuckoo hash table storing fingerprints (short hashes) of elements.

**Partitioned Bloom Filter:** Partition the m bits into k segments of size
m/k. Hash function i maps to a position in segment i. This improves cache
performance: each check accesses exactly k cache lines, while a standard
Bloom filter might access k different random cache lines.

**Distributed Bloom Filter:** Split the bit array across multiple machines
(consistent hashing on the bit positions). Allows Bloom filters that don't
fit in memory. Each node stores a shard; a lookup fans out to the k relevant
shards. Used in distributed databases for cross-node pre-filtering.

**Blocked Bloom Filter (Cache-Friendly):** Divide the m-bit array into
blocks of size equal to a CPU cache line (typically 512 bits / 64 bytes).
All k hash positions for a given element land in the *same* block. This
means each lookup/insert touches exactly ONE cache line — a significant
speedup in practice. The tradeoff: very slightly higher false positive rate
for the same m and k.
