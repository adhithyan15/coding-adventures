# coding_adventures_bitset_native

A Ruby native extension wrapping the `bitset` Rust crate via `ruby-bridge`. Provides a compact boolean array packed into 64-bit words for space-efficient storage and hardware-accelerated bulk bitwise operations.

## What it does

`Bitset` stores a sequence of bits packed into machine-word-sized integers (`u64`). Instead of using an entire byte per boolean, it packs 64 into a single word, achieving 8x space savings and enabling CPU-native AND/OR/XOR/NOT on 64 bits at once.

## How it fits in the stack

This package sits in the **native extensions** layer:

```
Ruby application code
    |
    v
coding_adventures_bitset_native  (this gem -- Ruby API)
    |
    v
bitset (Rust crate)              (implementation)
    |
    v
ruby-bridge (Rust crate)         (zero-dependency FFI to Ruby C API)
```

The `ruby-bridge` crate declares Ruby's C API via `extern "C"` -- no rb-sys, no Magnus, no bindgen required. The compiled `.so`/`.bundle`/`.dll` is loaded by Ruby's `require` mechanism.

## Installation

```ruby
# Gemfile
gem "coding_adventures_bitset_native"
```

Requires Rust toolchain (cargo) for compilation.

## Usage

```ruby
require "coding_adventures_bitset_native"

# Create a bitset with 100 bits (all initially zero)
bs = CodingAdventures::BitsetNative::Bitset.new(100)
bs.set(0)
bs.set(42)
bs.set(99)
bs.popcount  # => 3

# Create from an integer
bs = CodingAdventures::BitsetNative::Bitset.from_integer(0b10100101)
bs.each_set_bit  # => [0, 2, 5, 7]

# Create from a binary string
bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1010")
bs.test?(1)  # => true
bs.test?(0)  # => false

# Bulk bitwise operations (return new bitsets)
a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100)
b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)
a.and(b).to_integer    # => 8  (0b1000)
a.or(b).to_integer     # => 14 (0b1110)
a.xor(b).to_integer    # => 6  (0b0110)
a.not.to_integer       # => 3  (0b0011)
a.and_not(b).to_integer # => 4  (0b0100)

# Query operations
bs.any?      # at least one bit set?
bs.all?      # all bits set?
bs.none?     # no bits set?
bs.empty?    # zero-length bitset?
bs.len       # logical length
bs.capacity  # allocated bits (multiple of 64)

# Conversion
bs.to_integer     # => Integer or nil (nil if > 64 bits)
bs.to_binary_str  # => "1010"
bs.to_s           # => "Bitset(1010)"
```

## API Reference

### Class Methods

| Method | Description |
|--------|-------------|
| `Bitset.new(size)` | Create a bitset with `size` bits, all zero |
| `Bitset.from_integer(n)` | Create from a non-negative integer |
| `Bitset.from_binary_str(s)` | Create from a binary string like `"1010"` |

### Instance Methods

| Method | Description |
|--------|-------------|
| `set(i)` | Set bit `i` to 1 (auto-grows) |
| `clear(i)` | Set bit `i` to 0 (no-op beyond len) |
| `test?(i)` | Is bit `i` set? (false beyond len) |
| `toggle(i)` | Flip bit `i` (auto-grows) |
| `and(other)` | Bitwise AND (intersection) |
| `or(other)` | Bitwise OR (union) |
| `xor(other)` | Bitwise XOR (symmetric difference) |
| `not` | Bitwise NOT (complement) |
| `and_not(other)` | Set difference (self & ~other) |
| `popcount` | Count of set bits |
| `len` | Logical length |
| `capacity` | Allocated bits |
| `any?` | At least one bit set? |
| `all?` | All bits set? |
| `none?` | No bits set? |
| `empty?` | Zero-length? |
| `each_set_bit` | Array of set bit indices |
| `to_integer` | Convert to Integer (-1 if too large) |
| `to_binary_str` | Convert to binary string |
| `to_s` | Human-readable representation |
| `==` | Equality comparison |

## Development

```bash
bundle install
bundle exec rake compile  # Build the Rust extension
bundle exec rake test     # Run tests (compiles first)
bundle exec rake clean    # Remove build artifacts
```
