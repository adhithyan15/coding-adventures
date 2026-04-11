# hash-functions

Pure Python implementations of four widely-used non-cryptographic hash functions.

## What it does

Maps arbitrary bytes or strings to fixed-size unsigned integers.  The
functions here are the same algorithms found inside CPython's `dict`,
Redis's LRU eviction, Cassandra's consistent hashing, and Hadoop's
MapReduce partitioner.

## Implemented algorithms

| Function | Output bits | Style |
|---|---|---|
| `fnv1a_32` | 32 | Byte-at-a-time: XOR then multiply |
| `fnv1a_64` | 64 | Same as 32-bit with 64-bit constants |
| `djb2` | 64 | Shift-and-add: `hash = hash * 33 + byte` |
| `polynomial_rolling` | variable (mod) | Rolling polynomial over Mersenne prime |
| `murmur3_32` | 32 | Block hash: 4 bytes per round, fmix32 finalizer |

## Usage

```python
from hash_functions import fnv1a_32, fnv1a_64, djb2, polynomial_rolling, murmur3_32

# All functions accept bytes or str
fnv1a_32(b"hello")          # 1335831723
fnv1a_32("hello")           # same — str is UTF-8 encoded
fnv1a_64(b"hello")          # 64-bit output
djb2(b"abc")                # 193485963
polynomial_rolling(b"abc")  # Mersenne-prime modular hash
murmur3_32(b"abc", seed=0)  # 0xB3DD93FA = 3016911924

# Analysis utilities
from hash_functions import avalanche_score, distribution_test
score = avalanche_score(fnv1a_32, output_bits=32, sample_size=1000)
# ideal ≈ 0.5 (50% of output bits flip per single input bit change)

chi2 = distribution_test(fnv1a_32, [b"key1", b"key2", ...], num_buckets=100)
# ideal ≈ num_buckets - 1 (uniform distribution)
```

## Where it fits

```
DT17: hash-functions   ← this package
  ├── DT18: hash-map   (uses hash functions for index computation)
  ├── DT19: hash-set   (hash map with no values)
  ├── DT21: hyperloglog
  └── DT22: bloom-filter
```

## Running tests

```bash
uv venv .venv --quiet --no-project
uv pip install --python .venv -e .[dev] --quiet
uv run --no-project python -m pytest tests/ -v
```
