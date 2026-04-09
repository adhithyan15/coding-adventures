# coding-adventures-hyperloglog

HyperLogLog approximate cardinality estimation — DT21.

Estimates the number of *distinct* elements in a stream using O(1) memory,
regardless of how many elements you have seen.

## What is HyperLogLog?

A hash set tracking every unique visitor to a busy website would require
gigabytes of RAM. HyperLogLog answers the same question — "how many unique
visitors today?" — using about 12 KB with ±0.81% error.

Redis implements this algorithm as `PFADD` / `PFCOUNT` / `PFMERGE`.
("PF" honours Philippe Flajolet, who invented the algorithm in 2007.)

```
Hash Set:       [  "alice", "bob", ..., 1M users  ]   ~32 MB
HyperLogLog:    [  16,384 registers × 6 bits each  ]  ~12 KB
                   Estimate: 1,000,000 ± 0.81%       660,000× smaller
```

## Layer position

```
DT17: hash-functions      ← depends on (FNV-1a 64-bit)
DT21: hyperloglog         ← [THIS PACKAGE]
DT22: bloom-filter        ← related probabilistic structure (membership, not counting)
DT25: mini-redis          ← will use this for PFADD/PFCOUNT
```

## Installation

```bash
pip install coding-adventures-hyperloglog
```

## Usage

```python
from hyperloglog import HyperLogLog

# Create a sketch (precision=14 is the Redis default, ±0.81% error, ~12 KB)
hll = HyperLogLog(precision=14)

# Add elements — any type with a str() representation works
for user_id in user_stream:
    hll.add(user_id)

# Estimate distinct count
print(f"~{hll.count():,} unique users")  # e.g., ~1,000,000

# len() is a shorthand for count()
print(len(hll))
```

### Merging sketches (union)

```python
jan_hll = HyperLogLog(precision=14)
feb_hll = HyperLogLog(precision=14)

# ... fill each sketch from that month's event stream ...

# How many unique users were active in January OR February?
both_months = jan_hll.merge(feb_hll)
print(both_months.count())
```

Merge is O(m) and requires no access to the original data.
Both sketches must have the same precision.

### Choosing precision

| precision | registers | Memory   | Standard error |
|-----------|-----------|----------|----------------|
| 4         | 16        | 96 bits  | ±26.0%         |
| 8         | 256       | 1.5 KB   | ±6.5%          |
| 10        | 1,024     | 768 B    | ±3.25%         |
| 12        | 4,096     | 3 KB     | ±1.63%         |
| 14        | 16,384    | ~12 KB   | ±0.81% (Redis) |
| 16        | 65,536    | ~48 KB   | ±0.41%         |

```python
# Find the smallest precision that achieves a desired error rate
p = HyperLogLog.optimal_precision(0.01)  # → 14 (achieves 0.81% < 1%)
```

## Algorithm overview

Each element is hashed to a 64-bit integer using FNV-1a 64-bit. The hash is
split into two parts:

```
64-bit hash:
┌────────────────┬──────────────────────────────────────────────────────┐
│  top b bits    │                bottom 64-b bits                      │
│  → bucket j    │  → ρ = count_leading_zeros + 1                       │
└────────────────┴──────────────────────────────────────────────────────┘
```

Register `M[j]` stores the maximum ρ seen for bucket j. Cardinality is
estimated via the harmonic mean of 2^M[j] across all registers, multiplied
by a bias-correction constant α, with small-range (LinearCounting) and
large-range corrections applied automatically.

## API reference

```python
class HyperLogLog:
    def __init__(self, precision: int = 14) -> None: ...
    def add(self, element: Any) -> None: ...
    def count(self) -> int: ...
    def merge(self, other: HyperLogLog) -> HyperLogLog: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

    @property
    def precision(self) -> int: ...

    @property
    def num_registers(self) -> int: ...

    @property
    def error_rate(self) -> float: ...

    @staticmethod
    def error_rate_for_precision(precision: int) -> float: ...

    @staticmethod
    def memory_bytes(precision: int) -> int: ...

    @staticmethod
    def optimal_precision(desired_error: float) -> int: ...
```

## Running tests

```bash
uv venv .venv --no-project
uv pip install --python .venv -e ../hash-functions
uv pip install --python .venv -e .[dev]
uv run --no-project python -m pytest tests/ -v
```

## Related packages

- `coding-adventures-hash-functions` (DT17) — FNV-1a 64-bit hash used internally
- `coding-adventures-bloom-filter` (DT22) — approximate set membership (planned)
