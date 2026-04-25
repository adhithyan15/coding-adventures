# bloom-filter — Java

A space-efficient probabilistic set membership filter. Answers "Have I seen this element before?" with two possible responses:

- **"Definitely NO"** — zero false negatives. Trust this completely.
- **"Probably YES"** — bounded false positive rate, tunable at construction.

## Usage

```java
import com.codingadventures.bloomfilter.BloomFilter;

// Create a filter for 1,000 elements with 1% false positive rate
BloomFilter<String> bf = new BloomFilter<>(1000, 0.01);

bf.add("alice");
bf.add("bob");

bf.contains("alice");  // true  — definitely was added
bf.contains("carol");  // false — definitely not added (zero false negatives)
bf.contains("dave");   // false or true — if true, it's a false positive (~1% chance)
```

## How It Works

The filter uses a bit array of `m` bits and `k` hash functions:

- **Add**: compute k bit positions; set each bit to 1.
- **Check**: compute the same k positions; if ALL are 1, return "probably yes"; if ANY is 0, return "definitely no".

Bit positions are computed using double hashing:

```
g_i(x) = (h1(x) + i × h2(x)) mod m   for i = 0, 1, ..., k-1
```

where `h1 = FNV-1a 32-bit` and `h2 = DJB2`, both decorated with the MurmurHash3 `fmix32` finalizer to break prefix correlation.

## Optimal Parameters

```java
// How many bits do I need?
long m = BloomFilter.optimalM(1_000_000, 0.01);  // ≈ 9,585,059

// How many hash functions?
int k = BloomFilter.optimalK(m, 1_000_000);      // ≈ 7

// How many elements fit in 1 MB at 1% FPR?
long cap = BloomFilter.capacityForMemory(1_000_000, 0.01);  // ≈ 877,000
```

## Memory vs Exact Sets

| FPR | Bits/element | Memory (1M items) |
|-----|-------------|-------------------|
| 10% | 4.79 | ~585 KB |
| 1%  | 9.58 | ~1.14 MB |
| 0.1% | 14.38 | ~1.72 MB |
| HashSet | ~320 | ~40 MB |

## Running Tests

```bash
gradle test
```
