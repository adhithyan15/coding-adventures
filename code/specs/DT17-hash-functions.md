# DT17 — Hash Functions

## Overview

A **hash function** maps arbitrary-length input data to a fixed-size integer
output called a **hash** or **digest**. Given bytes of any size — a single
character, a 10 GB file, a JSON document — the hash function produces a number
that fits in 32 or 64 bits.

This single idea underlies an enormous fraction of computing infrastructure:

- **Hash maps** (DT18) and hash sets (DT19) use hash functions to compute
  array indices in O(1)
- **Bloom filters** (DT22) and HyperLogLog (DT21) use them for probabilistic
  data structures
- **Password storage** — bcrypt/argon2 store `hash(password + salt)`, never
  the password itself
- **Content-addressable storage** — Git identifies every commit, file, and
  tree by its SHA-1/SHA-256 hash
- **Network integrity** — TCP checksums, TLS record MACs, file download
  verification
- **Hash flooding defense** (SipHash) — protects web servers against DoS
  attacks that craft colliding keys

This spec covers four real hash functions from scratch, going deep into the bit-
level math that makes each one work.

## Layer Position

```
(no DT prerequisites — this is a foundational spec)

DT17: hash-functions     ← [YOU ARE HERE]
  ├── DT18: hash-map     (uses hash functions for index computation)
  ├── DT19: hash-set     (hash map with no values)
  ├── DT21: hyperloglog  (probabilistic cardinality; uses hash outputs)
  └── DT22: bloom-filter (probabilistic membership; uses multiple hashes)
```

**Depends on:** none (pure bit manipulation, arithmetic).
**Used by:** DT18, DT19, DT21, DT22, DT25 (mini-redis).

## Concepts

### What Is a Hash Function?

Think of a hash function as a mixing machine:

```
Input:   "Hello, world!"                 (13 bytes)
         [48 65 6c 6c 6f 2c 20 77 6f 72 6c 64 21]
          ↓
         [=== hash function ===]
          ↓
Output:  0xC0535E4B                      (always 4 bytes / 32 bits)
         (3227534923 as unsigned decimal)
```

Two critical properties set hash functions apart from other functions:

1. **Fixed output size** — No matter how long the input, the output is always
   the same number of bits (32, 64, 128, 256, etc.).
2. **Deterministic** — The same input always produces the same output.

### Five Properties of Good Hash Functions

**1. Deterministic**
Same input → same output, always, everywhere, forever.

**2. Uniform distribution**
Outputs should be spread evenly across the entire output range [0, 2^32).
If you hash 1,000 keys and map them into 10 buckets, each bucket should get
roughly 100 keys. A bad hash function might put 900 keys in bucket 0.

```
Good distribution (approximate):
  bucket 0: ████████████ 102 keys
  bucket 1: ██████████   98 keys
  bucket 2: ███████████  101 keys
  ...

Bad distribution:
  bucket 0: ████████████████████████████████████████ 400 keys
  bucket 1: ████████████████████████ 240 keys
  bucket 2: ██ 20 keys
  ...
```

**3. Avalanche effect**
Changing a single bit in the input should flip approximately 50% of the
output bits. If changing "Hello" to "hello" (bit flip in first byte)
only changes a few bits in the output, attackers can predict relationships
between nearby keys.

```
Measuring avalanche for a 32-bit hash:
  Input "Hello":  hash = 0b10110101_00110010_11001000_01010111
  Input "hello":  hash = 0b01001010_11001101_00110111_10101000
                         (18/32 bits differ ≈ 56% ✓ good)

  Bad hash:
  Input "Hello":  hash = 0b10110101_00110010_11001000_01010111
  Input "hello":  hash = 0b10110101_00110010_11001000_01010110
                         (1/32 bits differ = 3% ✗ bad)
```

**4. Speed**
Non-cryptographic hash functions (FNV-1a, DJB2, MurmurHash3) should run in
nanoseconds. Cryptographic hash functions (SHA-256) are allowed to be slower
because security matters more than raw speed. SipHash sits in between.

**5. Collision resistance** (for cryptographic uses)
A collision is two inputs with the same output: `hash(A) == hash(B)` with
`A ≠ B`. For a 32-bit hash, pigeonhole guarantees collisions exist (there are
only 2^32 ≈ 4 billion possible outputs but infinitely many inputs). The goal is
to make finding a collision computationally infeasible.

Non-cryptographic hashes (FNV-1a, DJB2, MurmurHash3) do NOT guarantee
collision resistance — they are designed for speed and distribution only.

### Building Blocks: The Three Operations

Almost every hash function uses only three bit operations:

**XOR (⊕)** — mixes two values without losing information:
```
  0b1010 XOR 0b1100 = 0b0110
  
  Key property: XOR is its own inverse: (A XOR B) XOR B = A
  This means mixing a byte in via XOR is reversible — but mixed with
  multiply, it becomes hard to invert.
```

**Multiply (×)** — spreads a change in one bit across many bits:
```
  0b0001 × 0x9e3779b9 = 0x9e3779b9
  0b0010 × 0x9e3779b9 = 0x3c6ef372  (completely different bit pattern)
  
  A low bit affects all higher bits via carry propagation.
```

**Bit shift (>> or <<)** — moves bits to a different position:
```
  0b10110100 >> 3 = 0b00010110  (lost 3 low bits, zeros filled on left)
  
  XOR-then-shift is a common "mixer": h ^= h >> 16
  This folds the high bits down onto the low bits, ensuring high bits
  influence the final output even after further operations.
```

---

### Hash Function 1: FNV-1a (Fowler-Noll-Vo)

FNV-1a is simple, fast, and widely used. It processes one byte at a time.

**Algorithm:**
```
hash = FNV_OFFSET_BASIS
for each byte b in input:
    hash = hash XOR b
    hash = hash × FNV_PRIME
return hash
```

**Constants (32-bit variant):**
```
FNV_OFFSET_BASIS = 2166136261  (0x811c9dc5)
FNV_PRIME        = 16777619    (0x01000193)
```

**Constants (64-bit variant):**
```
FNV_OFFSET_BASIS = 14695981039346656037  (0xcbf29ce484222325)
FNV_PRIME        = 1099511628211         (0x00000100000001b3)
```

**Why XOR-then-multiply?**
- XOR mixes the new byte into the hash. If you only XORed, you could undo the
  operation by XORing again.
- The multiply then "spreads" that byte's bits across the entire 32-bit hash.
  The FNV prime is specially chosen so that the multiplication creates a good
  avalanche without being too expensive.

**Why that specific prime?**
The FNV prime 16777619 in binary is:
```
0b00000001_00000000_00000001_10010011
```
It has a special property: multiplying by it is equivalent to several shifts
and additions, which CPUs execute very fast. More importantly, it was empirically
selected to produce good distribution on common hash table inputs (short strings,
integers, file paths).

**Bit-level trace for hashing "abc":**
```
Step 0: hash = 2166136261           (0x811c9dc5)

Byte 'a' = 0x61:
  hash ^= 0x61 → 0x811c9dc5 ^ 0x61 = 0x811c9da4
  hash  *= FNV_PRIME
       = 0x811c9da4 * 0x01000193
       = 0xe40c292c  (32-bit truncated)

Byte 'b' = 0x62:
  hash ^= 0x62 → 0xe40c292c ^ 0x62 = 0xe40c294e
  hash  *= FNV_PRIME
       = 0xe40c294e * 0x01000193
       = 0x4b9be1a3  (32-bit truncated)

Byte 'c' = 0x63:
  hash ^= 0x63 → 0x4b9be1a3 ^ 0x63 = 0x4b9be1c0
  hash  *= FNV_PRIME
       = 0x4b9be1c0 * 0x01000193
       = 0x1a47e90b  (32-bit truncated)

FNV-1a("abc") = 0x1a47e90b
```

**Use cases:** hash tables in networking tools, hash maps in Go (before SipHash),
file checksums in Mercurial. Excellent for short string keys.

---

### Hash Function 2: DJB2 (Dan Bernstein)

DJB2 is even simpler than FNV-1a and designed for maximum speed:

```
hash = 5381
for each byte b in input:
    hash = ((hash << 5) + hash) + b    # equivalent to: hash = hash * 33 + b
return hash
```

**Why 33?**
The inner expression `(hash << 5) + hash` computes `hash × 2^5 + hash = hash × 33`.
Why not just write `hash * 33`? Because on many architectures (especially older
ones without hardware multiply), a shift-plus-add is a single instruction while
multiply may not be. Even on modern CPUs, this form can be faster due to
pipelining.

33 is also a good multiplier because it is prime, and the bit pattern of the
multiplication provides decent avalanche.

**Why 5381?**
It is an arbitrary prime chosen by Bernstein through empirical testing — it
produces fewer collisions than nearby numbers for the benchmark inputs he cared
about (Unix dictionary words, C identifiers). Do not read too much into it.

**Trace for "abc":**
```
hash = 5381  = 0x1505

Byte 'a' = 97:
  hash = (0x1505 << 5) + 0x1505 + 97
       = 0x2a0a0 + 0x1505 + 97
       = 0x2b5a5 + 97
       = 0x2b606  = 177670

Byte 'b' = 98:
  hash = (177670 << 5) + 177670 + 98
       = 5685440 + 177670 + 98
       = 5863208

Byte 'c' = 99:
  hash = (5863208 << 5) + 5863208 + 99
       = 187622656 + 5863208 + 99
       = 193485963

DJB2("abc") = 193485963
```

**Use cases:** classic Unix `hash` in shells, many scripting language hash
tables historically. Extremely simple to implement correctly.

---

### Hash Function 3: MurmurHash3

MurmurHash3 is a modern, high-quality non-cryptographic hash designed by
Austin Appleby (2008). It is far better than FNV-1a or DJB2 at the avalanche
property, and faster in practice on 64-bit CPUs because it processes 4 bytes
(one 32-bit word) at a time.

**32-bit variant, processing 4 bytes per round:**

```
murmur3_32(data, seed=0):
  c1 = 0xcc9e2d51
  c2 = 0x1b873593
  h  = seed

  # Process 4-byte blocks
  for each 4-byte block [b0, b1, b2, b3]:
    k = (b3 << 24) | (b2 << 16) | (b1 << 8) | b0  # little-endian word
    k *= c1
    k  = rotl32(k, 15)    # rotate left 15 bits
    k *= c2
    h ^= k
    h  = rotl32(h, 13)
    h  = h * 5 + 0xe6546b64

  # Handle remaining 1-3 bytes (the "tail")
  k = 0
  for each remaining byte at offset i:
    k |= byte << (i * 8)
  k *= c1
  k  = rotl32(k, 15)
  k *= c2
  h ^= k

  # Finalization mixing
  h ^= len(data)
  h  = fmix32(h)
  return h
```

**The finalization mixer fmix32 — the key to avalanche:**

```
fmix32(h):
  h ^= h >> 16
  h *= 0x85ebca6b
  h ^= h >> 13
  h *= 0xc2b2ae35
  h ^= h >> 16
  return h
```

Why does this achieve full avalanche? Let's trace one bit flip through fmix32.

Consider bit 0 (the least significant bit) being set:
```
h = 0x00000001

Step 1: h ^= h >> 16
  h >> 16 = 0x00000000  (bit 0 doesn't reach the top half)
  h = 0x00000001        (bit 0 still isolated)

Step 2: h *= 0x85ebca6b
  0x00000001 * 0x85ebca6b = 0x85ebca6b
  Now bits are spread to positions 0, 1, 3, 5, 6, 9, 10, 11... (the prime's bit pattern)

Step 3: h ^= h >> 13
  0x85ebca6b >> 13 = 0x000042f5
  0x85ebca6b ^ 0x000042f5 = 0x85eb889e
  High bits now contaminate mid bits.

Step 4: h *= 0xc2b2ae35
  Full 32-bit multiplication — all bits intermingled.

Step 5: h ^= h >> 16
  Folds top 16 bits onto bottom 16 bits — ensures every bit of output
  depends on every bit that existed after step 4.
```

After 5 operations, a change in any single input bit affects all 32 output
bits. This is the **strict avalanche criterion** (SAC): every output bit
depends on every input bit.

**Why rotl32 (rotate left)?**
Unlike shift, which discards bits, rotation wraps them around:
```
rotl32(0b10000000_00000000_00000000_00000001, 1)
      = 0b00000000_00000000_00000000_00000011
```
Rotation keeps all bits in play while changing their position — important
for the mixing stage where we want bits from one part of the word to
influence another.

**Use cases:** Hadoop (Java), Clojure, Cassandra, Redis (for LRU eviction),
Python `struct.pack` benchmarks. The gold standard for non-cryptographic hash
tables.

---

### Hash Function 4: SipHash-2-4

SipHash, designed by Jean-Philippe Aumasson and Daniel J. Bernstein (2012),
is a **keyed** hash function. It takes a 128-bit secret key in addition to
the input, making it infeasible for an attacker to predict hash values.

**Why does this matter? Hash flooding attacks.**

In 2011, researchers demonstrated that many web frameworks (Ruby on Rails,
Python's Django, PHP, Java) were vulnerable to DoS attacks via hash table
degradation:

```
Attack:
1. Attacker crafts 100,000 POST parameters whose names all hash to the
   same bucket (they can do this because the hash function is public).
2. Server creates a hash map to parse the request.
3. All 100,000 parameters land in one bucket → O(n²) lookup time.
4. Single HTTP request takes 100+ seconds of CPU → server DoS.
```

With SipHash, the attacker cannot predict hash values because they do not know
the secret key (generated at process startup). Even if they could observe all
hash outputs, the 128-bit key space makes brute-force key recovery infeasible.

**SipHash-2-4 (2 compression rounds, 4 finalization rounds):**

```
siphash_2_4(data, key):
  # Split 128-bit key into two 64-bit words
  k0 = key[0:8]  as little-endian uint64
  k1 = key[8:16] as little-endian uint64

  # Initialize state (four 64-bit words)
  v0 = k0 ^ 0x736f6d6570736575
  v1 = k1 ^ 0x646f72616e646f6d
  v2 = k0 ^ 0x6c7967656e657261
  v3 = k1 ^ 0x7465646265656665

  # Process 8-byte blocks
  for each 8-byte block m:
    v3 ^= m
    sipround(); sipround()    # 2 compression rounds
    v0 ^= m

  # Process remaining bytes + length byte
  last_block = (len(data) % 256) << 56
  for each remaining byte at offset i: last_block |= byte << (i * 8)
  v3 ^= last_block
  sipround(); sipround()
  v0 ^= last_block

  # Finalization
  v2 ^= 0xff
  sipround(); sipround(); sipround(); sipround()  # 4 finalization rounds
  return v0 ^ v1 ^ v2 ^ v3
```

**One SipRound:**
```
sipround():
  v0 += v1; v1 = rotl64(v1, 13); v1 ^= v0
  v0 = rotl64(v0, 32)
  v2 += v3; v3 = rotl64(v3, 16); v3 ^= v2
  v0 += v3; v3 = rotl64(v3, 21); v3 ^= v0
  v2 += v1; v1 = rotl64(v1, 17); v1 ^= v2
  v2 = rotl64(v2, 32)
```

Each round is a sequence of add-rotate-xor (ARX) operations — the same
building block used in ChaCha20 (a stream cipher). Four 64-bit state words
interact so that a change in any bit of any word propagates to all other
words within one or two rounds.

**Used by:** Python (since 3.3), Ruby (since 1.9.3 with security patches),
Rust's `HashMap`, Perl 5.18+, Erlang/Elixir.

---

### Distribution Analysis: How to Test a Hash Function

**Chi-squared test for uniformity:**

Given n inputs hashed into k buckets, the expected count per bucket is n/k.
The chi-squared statistic measures how far the actual distribution deviates:

```
χ² = Σ (observed_i - expected)² / expected
     for each bucket i

If χ² ≈ k-1: good distribution (matches theoretical chi-squared distribution)
If χ² >> k:  poor distribution (some buckets overcrowded)
```

**Avalanche analysis:**

```
For each input bit position b (0 to 7 for single-byte input):
  For N random inputs:
    flip bit b in the input
    hash both the original and flipped input
    count how many output bits differ
  avalanche_score[b] = average bits changed / total_output_bits
  ideal: avalanche_score[b] ≈ 0.5
```

A hash function with strict avalanche: every `avalanche_score[b]` is close
to 0.5. A hash function with weak avalanche: some bits have scores near 0 or
1, meaning they barely affect (or always affect) certain output bits.

## Representation

Hash functions are pure functions — they have no mutable state (except for
the secret key in SipHash, which is set once at initialization).

```
HashState (for streaming APIs):
  algorithm: str         # "fnv1a32", "djb2", "murmur3_32", "siphash_2_4"
  accumulator: int       # current hash value
  buffer: bytes          # partial block (for block-oriented hashes)
  length: int            # bytes processed so far (for finalization)
  key: bytes | None      # 128-bit key for SipHash; None otherwise
```

For non-streaming use (hash the whole input at once), no state object is
needed — just call the function directly.

## Algorithms (Pure Functions)

### `fnv1a_32(data: bytes) → int`

```
hash = 2166136261
for b in data:
    hash = ((hash ^ b) * 16777619) & 0xFFFFFFFF
return hash
```

### `fnv1a_64(data: bytes) → int`

```
hash = 14695981039346656037
for b in data:
    hash = ((hash ^ b) * 1099511628211) & 0xFFFFFFFFFFFFFFFF
return hash
```

### `djb2(data: bytes) → int`

```
hash = 5381
for b in data:
    hash = (((hash << 5) + hash) + b) & 0xFFFFFFFFFFFFFFFF
return hash
```

### `murmur3_32(data: bytes, seed: int = 0) → int`

See full pseudocode in Concepts section. Key steps:
1. Process 4-byte blocks with c1, rotl, c2, XOR into h.
2. Handle tail bytes.
3. XOR with length, apply fmix32.

### `siphash_2_4(data: bytes, key: bytes) → int`

See full pseudocode in Concepts section.
- key must be exactly 16 bytes.
- Returns a 64-bit integer.

### `avalanche_score(hash_fn, sample_size: int = 1000) → float`

```
total_bit_flips = 0
total_trials = 0
for _ in range(sample_size):
    input_bytes = random_bytes(8)
    h1 = hash_fn(input_bytes)
    for bit in range(len(input_bytes) * 8):
        flipped = flip_bit(input_bytes, bit)
        h2 = hash_fn(flipped)
        diff = h1 ^ h2
        total_bit_flips += popcount(diff)  # count set bits
        total_trials += output_bits(hash_fn)
return total_bit_flips / total_trials      # ideal: 0.5
```

### `distribution_test(hash_fn, inputs, num_buckets) → float`

Returns the chi-squared statistic (lower = better; ideal ≈ num_buckets - 1).

```
counts = [0] * num_buckets
for inp in inputs:
    bucket = hash_fn(inp) % num_buckets
    counts[bucket] += 1
expected = len(inputs) / num_buckets
chi2 = sum((c - expected)**2 / expected for c in counts)
return chi2
```

## Public API

```python
# All functions take bytes as input, return unsigned int

def fnv1a_32(data: bytes) -> int: ...            # 32-bit output
def fnv1a_64(data: bytes) -> int: ...            # 64-bit output
def djb2(data: bytes) -> int: ...                # 64-bit output (no truncation)
def murmur3_32(data: bytes, seed: int = 0) -> int: ...   # 32-bit output
def siphash_2_4(data: bytes, key: bytes) -> int: ...     # 64-bit output; key must be 16 bytes

# Analysis utilities
def avalanche_score(
    hash_fn: Callable[[bytes], int],
    output_bits: int,
    sample_size: int = 1000
) -> float: ...

def distribution_test(
    hash_fn: Callable[[bytes], int],
    inputs: list[bytes],
    num_buckets: int
) -> float: ...     # returns chi-squared statistic

# Convenience: hash a string (UTF-8 encoded)
def hash_str_fnv1a_32(s: str) -> int: ...
def hash_str_siphash(s: str, key: bytes) -> int: ...
```

## Composition Model

### Inheritance languages (Python, Ruby, TypeScript)

```python
# Python — abstract base class for hash functions
from abc import ABC, abstractmethod

class HashFunction(ABC):
    @abstractmethod
    def hash(self, data: bytes) -> int: ...
    @abstractmethod
    def output_bits(self) -> int: ...

class FNV1a32(HashFunction):
    OFFSET = 2166136261
    PRIME  = 16777619
    def hash(self, data: bytes) -> int:
        h = self.OFFSET
        for b in data:
            h = ((h ^ b) * self.PRIME) & 0xFFFFFFFF
        return h
    def output_bits(self) -> int: return 32

class SipHash24(HashFunction):
    def __init__(self, key: bytes):
        assert len(key) == 16
        self._key = key
    def hash(self, data: bytes) -> int: ...
    def output_bits(self) -> int: return 64
```

### Composition languages (Rust, Go, Elixir, Lua, Perl, Swift)

```rust
// Rust — trait-based polymorphism
pub trait HashFunction {
    fn hash(&self, data: &[u8]) -> u64;
    fn output_bits(&self) -> u32;
}

pub struct Fnv1a32;
impl HashFunction for Fnv1a32 {
    fn hash(&self, data: &[u8]) -> u64 {
        let mut h: u32 = 2166136261;
        for &b in data {
            h ^= b as u32;
            h = h.wrapping_mul(16777619);
        }
        h as u64
    }
    fn output_bits(&self) -> u32 { 32 }
}

pub struct SipHash24 {
    key: [u8; 16],
}
impl HashFunction for SipHash24 {
    fn hash(&self, data: &[u8]) -> u64 { ... }
    fn output_bits(&self) -> u32 { 64 }
}
```

```go
// Go — interface
type HashFunction interface {
    Hash(data []byte) uint64
    OutputBits() int
}

type FNV1a32 struct{}
func (FNV1a32) Hash(data []byte) uint64 { ... }
func (FNV1a32) OutputBits() int { return 32 }

type SipHash24 struct{ Key [16]byte }
func (s SipHash24) Hash(data []byte) uint64 { ... }
func (s SipHash24) OutputBits() int { return 64 }
```

```elixir
# Elixir — behaviour
defmodule HashFunction do
  @callback hash(binary()) :: non_neg_integer()
  @callback output_bits() :: pos_integer()
end

defmodule FNV1a32 do
  @behaviour HashFunction
  @offset 2_166_136_261
  @prime  16_777_619
  def hash(data) do
    Enum.reduce(:binary.bin_to_list(data), @offset, fn b, h ->
      Bitwise.band(Bitwise.*(Bitwise.bxor(h, b), @prime), 0xFFFFFFFF)
    end)
  end
  def output_bits(), do: 32
end
```

## Test Strategy

### Unit tests

```
# FNV-1a 32-bit (known vectors)
fnv1a_32(b"")         → 2166136261
fnv1a_32(b"a")        → 84696351
fnv1a_32(b"abc")      → 440920331   (0x1a47e90b)
fnv1a_32(b"foobar")   → 2984838064

# FNV-1a 64-bit
fnv1a_64(b"")         → 14695981039346656037
fnv1a_64(b"a")        → 12638187200555641996

# DJB2
djb2(b"")             → 5381
djb2(b"a")            → 177670
djb2(b"abc")          → 193485963

# MurmurHash3 (known test vectors from reference implementation)
murmur3_32(b"", seed=0)       → 0
murmur3_32(b"", seed=1)       → 0x514e28b7
murmur3_32(b"a", seed=0)      → 0xe40c292c
murmur3_32(b"abc", seed=0)    → 0xb3dd93fa

# SipHash-2-4 (from the reference test vectors in the SipHash paper)
key = bytes(range(16))    # [0x00, 0x01, ..., 0x0f]
siphash_2_4(b"", key)     → 0x726fdb47dd0e0e31
siphash_2_4(b"\x00", key) → 0x74f839c593dc67fd

# Avalanche tests
avalanche_score(fnv1a_32, 32)  → between 0.40 and 0.60
avalanche_score(murmur3_32, 32) → between 0.45 and 0.55
avalanche_score(siphash_2_4, 64) → between 0.47 and 0.53

# Distribution tests (chi-squared)
# 10,000 random strings, 100 buckets → chi-squared should be near 99
distribution_test(fnv1a_32, random_strings(10000), 100) < 200
distribution_test(murmur3_32, random_strings(10000), 100) < 150
```

### Property-based tests

- `fnv1a_32(data)` always returns a value in [0, 2^32)
- `siphash_2_4(data, key1)` ≠ `siphash_2_4(data, key2)` with high probability when key1 ≠ key2
- For all hash functions: `hash(data) == hash(data)` (determinism)
- Two different inputs very rarely produce the same 32-bit hash
  (test with 100,000 random inputs; expect 0 or 1 collision)

## Future Extensions

- **SHA-256 and SHA-3**: cryptographic hash functions suitable for
  password hashing, digital signatures, blockchain. Cover in a separate
  cryptography spec.
- **xxHash**: faster than MurmurHash3 on modern CPUs with SIMD; used in
  LZ4, ClickHouse, Hadoop 3.
- **CityHash / FarmHash**: Google's hash functions; tuned for short string
  keys on 64-bit CPUs.
- **BLAKE3**: modern cryptographic hash; competitive speed with xxHash;
  parallelizable with SIMD.
- **Consistent hashing**: distributes keys across a ring of nodes (used in
  distributed systems like DynamoDB, Cassandra) — builds on hash functions.
