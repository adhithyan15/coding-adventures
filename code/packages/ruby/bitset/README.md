# coding_adventures_bitset

A compact bitset data structure that packs boolean values into 64-bit integers, providing O(n/64) bulk bitwise operations.

## What is a Bitset?

A bitset stores a sequence of bits -- each one either 0 or 1 -- packed into machine-word-sized integers. Instead of using an entire object reference to represent a single true/false value, a bitset packs 64 of them into a single `Integer`.

- **Space**: 64x more compact than an Array of booleans
- **Speed**: Bulk operations (AND, OR, XOR, NOT) process 64 bits per CPU instruction
- **Applications**: Bloom filters, graph algorithms, bitmap indexes, register allocation

## Installation

```ruby
gem "coding_adventures_bitset"
```

## Usage

```ruby
require "coding_adventures_bitset"

Bitset = CodingAdventures::Bitset::Bitset

# Create a bitset and set some bits
bs = Bitset.new(100)
bs.set(0).set(42).set(99)
bs.popcount  # => 3

# Test and clear bits
bs.test?(42)  # => true
bs.clear(42)
bs.test?(42)  # => false

# Iterate over set bits
bs.each_set_bit { |i| puts i }
# Output: 0, 99

# Bulk bitwise operations
a = Bitset.from_integer(0b1100)
b = Bitset.from_integer(0b1010)
(a & b).to_integer  # => 8  (0b1000, intersection)
(a | b).to_integer  # => 14 (0b1110, union)
(a ^ b).to_integer  # => 6  (0b0110, symmetric difference)
(~a).to_integer     # => 3  (0b0011, complement within len)

# Create from binary string
bs = Bitset.from_binary_str("10110")
bs.to_integer    # => 22
bs.to_binary_str # => "10110"

# Auto-growth (ArrayList-style doubling)
bs = Bitset.new(10)
bs.set(1000)    # automatically grows
bs.size         # => 1001
```

## Layer Position

The bitset is a standalone foundation package with no dependencies. It sits beneath higher-level data structures like Bloom filters, bitmap indexes, and adjacency matrices.

## API Reference

### Constructors

- `Bitset.new(size)` -- Create with all bits zero
- `Bitset.from_integer(value)` -- Create from non-negative integer
- `Bitset.from_binary_str(str)` -- Create from "0"/"1" string

### Single-Bit Operations

- `set(i)` -- Set bit i to 1 (auto-grows)
- `clear(i)` -- Set bit i to 0 (no-op if beyond len)
- `test?(i)` / `test(i)` -- Check if bit i is set
- `toggle(i)` -- Flip bit i (auto-grows)

### Bulk Bitwise Operations

- `bitwise_and(other)` / `&` -- Intersection
- `bitwise_or(other)` / `|` -- Union
- `bitwise_xor(other)` / `^` -- Symmetric difference
- `bitwise_not` / `~` -- Complement
- `and_not(other)` -- Set difference

### Counting and Query

- `popcount` -- Count of set bits
- `size` -- Logical length
- `capacity` -- Allocated bits
- `any?` / `all?` / `none?` / `empty?`

### Iteration

- `each_set_bit { |i| ... }` -- Yields indices of set bits

### Conversion

- `to_integer` -- Convert to Integer
- `to_binary_str` -- Convert to "0"/"1" string
- `to_s` -- Human-readable "Bitset(101)"

## Development

```bash
bundle install
bundle exec rake test
```
