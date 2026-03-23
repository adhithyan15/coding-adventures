# coding-adventures-bitset

A compact bitset data structure that packs boolean values into 64-bit words.

## Overview

A bitset stores a sequence of bits -- each one either 0 or 1 -- packed into
machine-word-sized integers. Instead of using an entire Python `bool` object
(~28 bytes on CPython) to represent a single true/false value, a bitset packs
64 of them into a single `int`.

**Space**: 10,000 booleans as `list[bool]` consume ~80,000 bytes. A bitset
storing the same 10,000 bits uses ~1,250 bytes -- a 64x improvement.

**Speed**: AND-ing two boolean lists loops over 10,000 elements. AND-ing two
bitsets loops over ~157 words, performing a single 64-bit AND per iteration.

## Layer Position

The bitset is a standalone foundation package with no dependencies. It sits
beneath higher-level data structures like Bloom filters, bitmap indexes, and
adjacency matrices.

## Installation

```bash
pip install coding-adventures-bitset
```

## Usage

```python
from bitset import Bitset

# Create a bitset and set some bits
bs = Bitset(100)
bs.set(0)
bs.set(42)
bs.set(99)
assert bs.popcount() == 3

# Test individual bits
assert bs.test(42)
assert not bs.test(50)

# Iterate over set bits (efficient -- skips zero words)
print(list(bs.iter_set_bits()))  # [0, 42, 99]

# Bulk bitwise operations return new bitsets
other = Bitset(100)
other.set(42)
other.set(50)
intersection = bs & other          # bitwise AND
union = bs | other                 # bitwise OR
symmetric_diff = bs ^ other        # bitwise XOR
complement = ~bs                   # bitwise NOT
difference = bs.and_not(other)     # set difference

# Construct from integers or binary strings
bs = Bitset.from_integer(42)       # binary: 101010
bs = Bitset.from_binary_str("101") # bits 0 and 2 set

# Convert back
print(bs.to_integer())             # 5
print(bs.to_binary_str())          # "101"

# Python protocols
assert 2 in Bitset.from_integer(5) # __contains__
assert len(Bitset(100)) == 100     # __len__
for bit in Bitset.from_integer(5): # __iter__
    print(bit)                     # 0, 2
```

## API Reference

### Constructors

- `Bitset(size=0)` -- Create with all bits zero
- `Bitset.from_integer(value)` -- Create from a non-negative integer
- `Bitset.from_binary_str(s)` -- Create from a `"01"` string (MSB-first)

### Single-Bit Operations

- `set(i)` -- Set bit i to 1 (auto-grows)
- `clear(i)` -- Set bit i to 0 (no-op if out of range)
- `test(i)` -- Test if bit i is 1 (False if out of range)
- `toggle(i)` -- Flip bit i (auto-grows)

### Bulk Bitwise Operations

All return new bitsets without modifying operands:

- `bitwise_and(other)` / `&` -- intersection
- `bitwise_or(other)` / `|` -- union
- `bitwise_xor(other)` / `^` -- symmetric difference
- `bitwise_not()` / `~` -- complement
- `and_not(other)` -- set difference (A & ~B)

### Counting and Queries

- `popcount()` -- Count of set bits
- `len(bs)` -- Logical size (highest addressable bit + 1)
- `capacity()` -- Allocated size in bits (multiple of 64)
- `any()` -- True if any bit is set
- `all()` -- True if all bits are set (vacuous truth for empty)
- `none()` -- True if no bits are set

### Iteration

- `iter_set_bits()` -- Yields indices of set bits in ascending order
- `for bit in bs` -- Same as `iter_set_bits()`

### Conversion

- `to_integer()` -- Convert to non-negative int
- `to_binary_str()` -- Convert to `"01"` string (MSB-first)

## Development

```bash
uv venv && uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
