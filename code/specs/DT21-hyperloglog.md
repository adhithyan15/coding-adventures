# DT21 — HyperLogLog

## Overview

**HyperLogLog** (HLL) answers the question: "How many *distinct* elements have
I seen?" using a tiny, fixed amount of memory — typically 12 KB — regardless
of whether you've seen 1,000 or 1,000,000,000 elements.

This is an *approximate* answer. With 12 KB (the Redis default), the error is
±0.81%. That means if the true count is 1,000,000, HyperLogLog will return
something between 991,900 and 1,008,100. For most real-world use cases
(counting unique visitors, unique queries, unique IP addresses), this accuracy
is more than sufficient.

The alternative — storing every distinct element in a hash set — would cost
~32 MB for 1 million strings. HyperLogLog uses 12 KB regardless.

```
Hash Set:         ┌──────────────────────────────────────────┐
                  │ "alice", "bob", "carol", ..., 1M entries │
                  │            ~32 MB                         │
                  └──────────────────────────────────────────┘

HyperLogLog:      ┌────────────────────────────────┐
                  │ 16,384 registers × 6 bits each  │
                  │         exactly 12 KB           │
                  │    estimate: 1,000,000 ± 0.81%  │
                  └────────────────────────────────┘
```

Redis implements HyperLogLog with the `PFADD` and `PFCOUNT` commands.
("PF" honors Philippe Flajolet, the mathematician who invented the algorithm.)

## Layer Position

```
DT17: hash-functions      ← core primitive (good hash function is critical)
DT18: hash-map            ← exact key-value counting
DT19: hash-set            ← exact set membership
DT21: hyperloglog         ← [YOU ARE HERE] approximate cardinality estimation
DT22: bloom-filter        ← approximate set membership (related probabilistic DS)

DT25: mini-redis          ← uses DT21 for PFADD/PFCOUNT
```

**Depends on:** DT17 (hash functions — must be high-quality with uniform output).
**Contrasts with:** DT19 (exact but O(n) space) and DT22 (membership, not counting).
**Used by:** Redis PFADD/PFCOUNT, Google BigQuery APPROX_COUNT_DISTINCT,
Facebook analytics, web analytics platforms, network monitoring.

## Concepts

### The Problem: Counting Distinct Elements

You have a stream of billions of events. You want to count unique users.
With a hash set: every new user ID is stored. Memory grows without bound.
With HyperLogLog: memory is fixed at 12 KB, regardless of stream size.

Why does this matter in practice?

```
Use case: How many unique IPs hit our CDN today?
  - A busy CDN serves 10 billion requests/day from ~500 million unique IPs
  - Hash set: 500M × 16 bytes (IPv6) = 8 GB of RAM just for one counter
  - HyperLogLog: 12 KB for ±0.81% accuracy — 660,000× smaller
```

### Step 1: The Probabilistic Foundation (Flajolet-Martin, 1985)

The magic comes from a beautiful observation about random binary strings.

If you hash each element to a random 64-bit binary string, each bit is equally
likely to be 0 or 1. The probability that the first bit is 0 is 1/2.
The probability that the first TWO bits are 0 is 1/4. The probability of
k leading zeros is 1/2^k.

Now, if you've seen n distinct elements and recorded the maximum number of
leading zeros across all their hash values, what do you expect that maximum
to be?

```
Think of it like flipping coins until you get HEADS.
  - P(0 leading zeros) = P(first bit is 1)     = 1/2     → common
  - P(1 leading zero)  = P(01...)               = 1/4     → less common
  - P(2 leading zeros) = P(001...)              = 1/8     → rare
  - P(k leading zeros)                          = 1/2^(k+1)

With n distinct elements, the maximum leading-zero count you'd expect
is roughly log₂(n). If the max you've seen is k, estimate n ≈ 2^k.
```

Worked example with 5 elements:

```
Element   Hash (binary)                 Leading zeros
-------   ----------------------------  -------------
"alice"   0010110101110010...           2
"bob"     0001111001000110...           3
"carol"   0110010101111010...           1
"dave"    0000101010010011...           4  ← maximum
"eve"     1001011010100100...           0

max_leading_zeros = 4
Estimate: n ≈ 2^4 = 16

Actual n = 5. Huge error! This simple estimator has high variance.
```

The estimate is terrible for small n and high variance for all n. But it gives
us the core intuition. The improvements that follow reduce the variance
dramatically.

### Step 2: LogLog — Reduce Variance with Multiple Registers

Instead of one global maximum, use m "registers" (buckets). Split each hash
into two parts:

```
64-bit hash of "alice":
┌─────────────────────────────────────────┐
│ first b bits │    remaining 64-b bits   │
│  (register   │    (count leading zeros  │
│   index j)   │     in this part)        │
└─────────────────────────────────────────┘

Example with b=4 (m=16 registers):
  hash("alice") = 0010 | 1101011100...
                   ↑         ↑
               j = 2     count leading zeros of "1101..." = 0
  So: registers[2] = max(registers[2], 0+1) = max(current, 1)
```

After processing all elements, average the estimates from all m registers.
More registers → lower variance → better accuracy.

With m registers and arithmetic mean:
  estimate = (m / Σ 2^(-registers[i]))  × some_constant

But arithmetic mean is sensitive to outliers (one register with a huge value
pulls the estimate way up). The fix: harmonic mean.

### Step 3: HyperLogLog — Use Harmonic Mean

HyperLogLog (Flajolet, Fusy, Gandouet, Meunier — 2007) replaces the arithmetic
mean with the **harmonic mean**. The harmonic mean de-emphasizes outliers:

```
Arithmetic mean of [1, 2, 3, 100]:
  (1 + 2 + 3 + 100) / 4 = 26.5   ← dominated by 100

Harmonic mean of [1, 2, 3, 100]:
  4 / (1/1 + 1/2 + 1/3 + 1/100) = 4 / (1 + 0.5 + 0.333 + 0.01) = 2.14
  ← resistant to the outlier
```

This single change — using harmonic mean instead of arithmetic mean — is the
key insight of HyperLogLog. It's why the algorithm achieves the theoretical
minimum error for a given memory budget.

### The Full Algorithm

```
PFADD(registers, element):
    h = hash64(element)            # 64-bit hash, must be high quality
    j = h >> (64 - b)              # take top b bits as register index
    w = h & ((1 << (64-b)) - 1)   # remaining 64-b bits
    leading_zeros = count_leading_zeros(w, 64-b) + 1
    registers[j] = max(registers[j], leading_zeros)

PFCOUNT(registers):
    m = 2^b                        # number of registers

    # Compute harmonic mean estimator
    Z = 0
    for i in range(m):
        Z += 2^(-registers[i])
    Z = 1.0 / Z                    # inverse of sum of 2^(-r)

    # Raw estimate
    alpha_m = correction_constant(m)
    E = alpha_m * m * m * Z

    # Apply small-range correction
    if E <= 2.5 * m:
        V = count_zero_registers(registers)
        if V > 0:
            E = m * log(m / V)     # LinearCounting for small cardinalities

    # Apply large-range correction
    if E > (1/30) * 2^64:
        E = -(2^64) * log(1 - E / 2^64)

    return round(E)
```

### The Bias Correction Constant (alpha_m)

The raw harmonic mean estimator is biased. It consistently overestimates
by a factor of alpha_m. The correction constants are derived analytically:

```
m (registers)   alpha_m
--------------  -------
16              0.673
32              0.697
64              0.709
128+            0.7213 / (1 + 1.079/m)  ← formula for larger m
```

These constants were derived by Philippe Flajolet using complex analysis.
You don't need to understand the derivation to use them — just know that
the raw estimate needs to be multiplied by alpha_m to remove the bias.

### Small-Range Correction: LinearCounting

When the estimated cardinality is small relative to m (< 2.5 × m), many
registers will still be at their initial value of 0. In this regime, the
harmonic mean estimator has poor accuracy.

LinearCounting is a separate, simpler algorithm that works well for small n:
count the number of *empty* registers V, and use:

```
E_small = m × ln(m / V)
```

This is accurate when V > 0 (i.e., not all registers have been set).
HyperLogLog automatically switches to LinearCounting when n is small.

```
Why this works:
  If we threw n balls into m bins uniformly at random, the expected number
  of empty bins is m × e^(-n/m). Solving for n: n = m × ln(m/V).
  This is the classic "coupon collector" approximation.
```

### Large-Range Correction

When n approaches 2^64 / 30, hash collisions start to cause systematic
underestimation (two distinct elements hash to the same value more often).
A correction is applied:

```
E_large = -(2^64) × ln(1 - E / 2^64)
```

In practice, this correction is almost never needed — 2^64 / 30 is about
600 quintillion, which is more distinct elements than any real application
processes.

### Error Rate vs Memory

The standard error of HyperLogLog is:

```
σ(relative) ≈ 1.04 / √m

where m = number of registers = 2^b

b (bits)   m (registers)   Memory      Standard Error
--------   -------------   ----------  ---------------
4          16              96 bits     26.0%
6          64              384 bits    13.0%
8          256             1.5 KB      6.5%
10         1,024           6 KB        3.25%
12         4,096           24 KB       1.625%
14         16,384          98 KB       0.8125%  ← Redis default
16         65,536          393 KB      0.406%
```

Redis uses b=14, m=16384. With 6 bits per register × 16384 = 98304 bits =
12 KB of memory, it achieves ±0.81% standard error.

Each register only needs to store values 0–63 (6 bits), because
the maximum meaningful leading-zero count in a 64-bit hash is 63.

### MERGE: Union of Two HyperLogLogs

Because each register stores the maximum leading-zero count seen for its
bucket, merging two HLLs is trivially O(m) — just take the element-wise max:

```
PFMERGE(hll1, hll2):
    result = new_hll(same_precision)
    for i in range(m):
        result.registers[i] = max(hll1.registers[i], hll2.registers[i])
    return result
```

This is the **union** operation. You get the count of distinct elements
across both data streams, without re-processing either stream.

There is NO intersection operation for HyperLogLog — only union.
(Intersection would require inclusion-exclusion which amplifies error.)

Real-world use: "How many unique users were active in either January OR
February?" — merge the two monthly HLLs and call PFCOUNT.

```
Jan HLL:  [3, 1, 5, 2, 4, ...]
Feb HLL:  [2, 4, 3, 1, 6, ...]
Merged:   [3, 4, 5, 2, 6, ...]  ← element-wise max
```

## Representation

```
HyperLogLog {
    precision: int          # b, number of bits for register index
    m: int                  # 2^b, number of registers
    registers: Array[int]   # m registers, each storing 0..63
                            # total storage: m * 6 bits
}

Example for b=14 (Redis default):
  m = 16384
  registers = [0, 0, 0, ..., 0]   # initially all zero
  total memory = 16384 * 6 bits = 98304 bits ≈ 12 KB
```

### Register Storage Optimization

Registers only hold values 0–63 (6 bits each). We can pack them tightly:

```
Naive:   m bytes (1 byte per register) → 16 KB for b=14
Packed:  m × 6 bits / 8 = 12 KB for b=14   ← Redis uses this

Packing example (3 registers × 6 bits = 18 bits = 3 bytes):
  reg[0] = 0b001010 = 10
  reg[1] = 0b000011 = 3
  reg[2] = 0b100001 = 33

  Packed bytes:
  byte 0: 001010 00      ← reg[0] + 2 bits of reg[1]
  byte 1: 1100 1000      ← 4 bits of reg[1] + 4 bits of reg[2]
  byte 2: 01 xxxxxx      ← 2 bits of reg[2] + padding
```

Redis actually uses a more complex sparse→dense representation:
- **Sparse** when few registers are set (< ~6KB worth of updates)
- **Dense** (packed 6-bit array) for the full representation

## Algorithms (Pure Functions)

### hash64(element) → int

Use a high-quality, fast hash function from DT17. MurmurHash3 or xxHash64
are common choices. The critical property: the output must be uniformly
distributed across all 64 bits.

```
# Bad choice: MD5 or SHA-1 — correct but slow (designed for security)
# Good choice: xxHash64, MurmurHash3, FNV-1a (designed for speed)
```

### count_leading_zeros(x, max_bits) → int

```
count_leading_zeros(x, max_bits=64):
    if x == 0:
        return max_bits   # all zeros
    count = 0
    bit = max_bits - 1
    while (x >> bit) & 1 == 0:
        count += 1
        bit -= 1
    return count

Example:
  x = 0b00101100... (leading 2 zeros)
  count_leading_zeros(x, 64) = 2
```

Most CPUs have a hardware instruction for this (CLZ on ARM, BSR/LZCNT on x86).
Python has no built-in, but we can implement it.

### add(hll, element) → HyperLogLog

```
add(hll, element):
    h = hash64(element)
    j = h >> (64 - hll.precision)          # top `b` bits → register index
    w = h & ((1 << (64 - hll.precision)) - 1)  # remaining bits
    rho = count_leading_zeros(w, 64 - hll.precision) + 1
    new_registers = copy(hll.registers)
    new_registers[j] = max(new_registers[j], rho)
    return HyperLogLog(precision=hll.precision, registers=new_registers)
```

### count(hll) → int

```
count(hll):
    m = hll.m
    # Harmonic mean sum
    Z_sum = sum(2.0 ** (-r) for r in hll.registers)
    Z = 1.0 / Z_sum
    alpha = alpha_m(m)
    E = alpha * m * m * Z

    # Small-range correction: use LinearCounting if many registers are 0
    V = hll.registers.count(0)
    if E <= 2.5 * m and V > 0:
        return round(m * math.log(m / V))

    # Large-range correction (rarely needed)
    two_64 = 2.0 ** 64
    if E > two_64 / 30.0:
        E = -two_64 * math.log(1.0 - E / two_64)

    return round(E)

alpha_m(m):
    if m == 16:   return 0.673
    if m == 32:   return 0.697
    if m == 64:   return 0.709
    return 0.7213 / (1 + 1.079 / m)   # for m >= 128
```

### merge(hll1, hll2) → HyperLogLog

```
merge(hll1, hll2):
    assert hll1.precision == hll2.precision, "Must have same precision"
    new_registers = [max(r1, r2) for r1, r2 in zip(hll1.registers, hll2.registers)]
    return HyperLogLog(precision=hll1.precision, registers=new_registers)
```

### error_rate(precision) → float

```
error_rate(precision):
    m = 2 ** precision
    return 1.04 / math.sqrt(m)

# Examples:
# error_rate(14) = 1.04 / sqrt(16384) ≈ 0.00812 = 0.81%
# error_rate(10) = 1.04 / sqrt(1024)  ≈ 0.0325  = 3.25%
```

## Public API

```python
class HyperLogLog:
    """
    Probabilistic cardinality estimator using O(1) memory.
    Estimates the number of distinct elements in a stream with
    configurable accuracy/memory tradeoff.

    Standard error ≈ 1.04 / sqrt(2^precision)
    Memory usage = 2^precision * 6 bits

    Redis default: precision=14, memory=12KB, error=0.81%
    """

    def __init__(self, precision: int = 14) -> "HyperLogLog":
        """
        Create an empty HyperLogLog.
        precision: b, where m=2^b registers are used.
        Valid range: 4 (26% error, 12B) to 18 (0.2% error, 196KB).
        """

    def add(self, element) -> "HyperLogLog":
        """
        Record that element has been seen.
        Returns a new HLL (functional style).
        O(1) — just one hash and one register update.
        """

    def count(self) -> int:
        """
        Estimate the number of distinct elements added so far.
        O(m) where m = number of registers.
        Error: approximately ±(1.04 / sqrt(m)) of the true count.
        Redis: PFCOUNT
        """

    def merge(self, other: "HyperLogLog") -> "HyperLogLog":
        """
        Return a new HLL representing the UNION of both sets.
        The result estimates distinct elements from either stream.
        O(m). No false negatives or positives for union — exact union
        is not possible, but the estimate has the same error rate.
        Redis: PFMERGE
        """

    @staticmethod
    def error_rate(precision: int) -> float:
        """
        Standard error rate for a given precision.
        Returns a fraction (e.g., 0.0081 for 0.81%).
        """

    @staticmethod
    def memory_bytes(precision: int) -> int:
        """
        Memory usage in bytes for a given precision.
        Returns 2^precision * 6 / 8 (packed representation).
        """

    @staticmethod
    def optimal_precision(desired_error: float) -> int:
        """
        Smallest precision that achieves the desired error rate.
        desired_error: fraction (e.g., 0.01 for 1%).
        """
```

## Composition Model

HyperLogLog composes on top of DT17 (hash functions). It is otherwise
self-contained — there is no other DT layer it wraps.

### Python / Ruby / TypeScript — Class with Register Array

```python
# Python: straightforward class
import math

class HyperLogLog:
    def __init__(self, precision=14):
        self.precision = precision
        self.m = 1 << precision               # 2^precision
        self.registers = [0] * self.m         # byte array, 0..63

    def add(self, element):
        import xxhash   # DT17 hash function
        h = xxhash.xxh64(str(element)).intdigest()
        j = h >> (64 - self.precision)        # top b bits
        w = h & ((1 << (64 - self.precision)) - 1)
        rho = self._leading_zeros(w, 64 - self.precision) + 1
        new_hll = HyperLogLog(self.precision)
        new_hll.registers = self.registers.copy()
        new_hll.registers[j] = max(new_hll.registers[j], rho)
        return new_hll

    @staticmethod
    def _leading_zeros(x, bits):
        if x == 0: return bits
        return bits - x.bit_length()
```

### Rust — Bit-Packed Registers

```rust
// Rust: bit-packed for memory efficiency, using byteorder crate
pub struct HyperLogLog {
    precision: u8,
    registers: Vec<u8>,  // packed: 6 bits per register
    m: usize,
}

impl HyperLogLog {
    pub fn add(&mut self, element: &[u8]) {
        let h = xxhash_rust::xxh64::xxh64(element, 0);
        let j = (h >> (64 - self.precision as u64)) as usize;
        let w = h << self.precision;
        let rho = w.leading_zeros() as u8 + 1;
        let current = self.get_register(j);
        if rho > current {
            self.set_register(j, rho);
        }
    }
}
```

### Go — Functional Style with Immutable Registers

```go
type HyperLogLog struct {
    Precision  uint8
    M          uint32
    Registers  []uint8  // one byte per register (wasteful but simple)
}

func (h HyperLogLog) Add(element []byte) HyperLogLog {
    hash := xxhash.Sum64(element)
    j := hash >> (64 - uint64(h.Precision))
    w := hash << h.Precision
    rho := uint8(bits.LeadingZeros64(w)) + 1

    newRegs := make([]uint8, len(h.Registers))
    copy(newRegs, h.Registers)
    if rho > newRegs[j] {
        newRegs[j] = rho
    }
    return HyperLogLog{Precision: h.Precision, M: h.M, Registers: newRegs}
}
```

### Elixir — Binary Pattern Matching on Hash Output

```elixir
defmodule HyperLogLog do
  defstruct precision: 14, registers: nil

  def new(precision \\ 14) do
    m = 1 <<< precision
    %HyperLogLog{precision: precision, registers: :array.new(m, default: 0)}
  end

  def add(%HyperLogLog{precision: b} = hll, element) do
    <<j::size(b), w::size(64 - b)>> = :xxhash.hash64(element)
    rho = leading_zeros(w, 64 - b) + 1
    current = :array.get(j, hll.registers)
    new_registers = :array.set(j, max(current, rho), hll.registers)
    %{hll | registers: new_registers}
  end

  defp leading_zeros(0, bits), do: bits
  defp leading_zeros(x, bits), do: bits - Integer.digits(x, 2) |> length()
end
```

## Test Strategy

### Accuracy at Various Cardinalities

```python
import random
import math

def test_accuracy():
    for n in [100, 1_000, 10_000, 100_000, 1_000_000]:
        hll = HyperLogLog(precision=14)
        for i in range(n):
            hll = hll.add(f"element_{i}")

        estimate = hll.count()
        error = abs(estimate - n) / n
        expected_error = 1.04 / math.sqrt(2**14)

        # With high probability, error is within 3 standard deviations
        assert error < 3 * expected_error, \
            f"n={n}: estimate={estimate}, error={error:.3%}"
        print(f"n={n}: estimate={estimate}, error={error:.3%}")
```

### Merge Correctness

```python
def test_merge():
    hll1 = HyperLogLog(precision=14)
    hll2 = HyperLogLog(precision=14)

    # Fill two non-overlapping sets
    for i in range(100_000):
        hll1 = hll1.add(f"a_{i}")
    for i in range(100_000):
        hll2 = hll2.add(f"b_{i}")

    merged = hll1.merge(hll2)
    estimate = merged.count()
    true_count = 200_000

    error = abs(estimate - true_count) / true_count
    assert error < 0.05   # within 5% (well above 3σ threshold)
```

### Small and Large Range Corrections

```python
def test_small_range():
    # Very few elements: LinearCounting should kick in
    hll = HyperLogLog(precision=14)
    for i in range(10):  # just 10 elements
        hll = hll.add(str(i))
    estimate = hll.count()
    assert 5 <= estimate <= 20   # rough bounds for very small n

def test_duplicate_inserts():
    # Adding the same element many times should not affect count
    hll = HyperLogLog(precision=14)
    for _ in range(10_000):
        hll = hll.add("same_element")
    assert hll.count() == 1   # only 1 distinct element
```

### Memory Invariant

```python
def test_memory():
    for precision in [4, 8, 10, 12, 14, 16]:
        hll = HyperLogLog(precision=precision)
        assert len(hll.registers) == 2**precision
        expected_bytes = (2**precision * 6) // 8
        # If using packed representation
        assert hll.memory_bytes(precision) == expected_bytes
```

### Error Rate Formula

```python
def test_error_rate():
    assert abs(HyperLogLog.error_rate(14) - 0.00812) < 0.0001
    assert abs(HyperLogLog.error_rate(10) - 0.03250) < 0.001
    assert HyperLogLog.optimal_precision(0.01) == 14  # for 1% error
```

## Future Extensions

**HyperLogLog++:** Google published improvements in 2013 (HLL++) that:
1. Use 64-bit hashes instead of 32-bit (eliminates large-range correction issues)
2. Apply empirical bias correction using precomputed lookup tables
3. Use a sparse representation for small cardinalities
This is what Google BigQuery uses internally.

**MinHash for Jaccard Similarity:** A related probabilistic data structure
estimates the *similarity* between two sets (Jaccard coefficient =
|A ∩ B| / |A ∪ B|) using random min-hash functions. Where HLL estimates
cardinality, MinHash estimates similarity. Both are used together in
recommendation systems.

**Streaming Quantile Sketches:** HyperLogLog solves the "count distinct"
problem. Related sketches solve other streaming aggregation problems:
- Count-Min Sketch: estimate frequency of any element (how many times did
  we see IP 1.2.3.4?)
- t-Digest: estimate quantiles (what's the 99th percentile response time?)
- KLL Sketch: mergeable quantile sketch with provable error bounds

**Adaptive Precision:** Start with low precision (cheap, low memory), and
automatically upgrade to higher precision if the cardinality estimate
exceeds a threshold. This keeps memory costs low for small datasets and
only spends memory when needed.
