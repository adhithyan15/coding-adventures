# Bitset (Elixir)

A compact bitset that packs boolean values into 64-bit integer words. Part of
the [coding-adventures](https://github.com/your-repo/coding-adventures) project.

## What is a Bitset?

A bitset stores a sequence of bits -- each one either 0 or 1 -- packed into
machine-word-sized integers. Instead of using an entire byte to represent a
single true/false value, a bitset packs 64 of them into a single word.

- **Space**: 64x more compact than a list of booleans
- **Speed**: Bulk operations (AND, OR, XOR) process 64 bits per instruction
- **Ubiquity**: Used in Bloom filters, graph algorithms, database indexes, etc.

## Layer Position

The bitset is a standalone foundation package with no dependencies. It sits
beneath higher-level data structures like Bloom filters, bitmap indexes, and
adjacency matrices.

## Installation

Add to your `mix.exs`:

```elixir
{:coding_adventures_bitset, path: "path/to/bitset"}
```

## Usage

```elixir
alias CodingAdventures.Bitset

# Create and manipulate
bs = Bitset.new(100)
     |> Bitset.set(0)
     |> Bitset.set(42)
     |> Bitset.set(99)

Bitset.popcount(bs)    # => 3
Bitset.set_bits(bs)    # => [0, 42, 99]
Bitset.test?(bs, 42)   # => true

# Bulk operations return new bitsets (immutable)
a = Bitset.from_integer(0b1100)
b = Bitset.from_integer(0b1010)
Bitset.to_integer(Bitset.bitwise_and(a, b))  # => 8 (0b1000)
Bitset.to_integer(Bitset.bitwise_or(a, b))   # => 14 (0b1110)

# Convert to/from strings
{:ok, bs} = Bitset.from_binary_str("1010")
Bitset.to_binary_str(bs)    # => "1010"
to_string(bs)               # => "Bitset(1010)"
```

## API

### Constructors

| Function | Description |
|---|---|
| `Bitset.new(size)` | Create a bitset with all bits initially zero |
| `Bitset.from_integer(value)` | Create from a non-negative integer |
| `Bitset.from_binary_str(str)` | Create from a binary string like `"1010"` |
| `Bitset.from_binary_str!(str)` | Like above but raises on invalid input |

### Single-Bit Operations

| Function | Description |
|---|---|
| `Bitset.set(bs, i)` | Set bit `i` to 1 (auto-grows) |
| `Bitset.clear(bs, i)` | Set bit `i` to 0 (no-op if beyond len) |
| `Bitset.test?(bs, i)` | Test if bit `i` is set |
| `Bitset.toggle(bs, i)` | Flip bit `i` (auto-grows) |

### Bulk Bitwise Operations

| Function | Description |
|---|---|
| `Bitset.bitwise_and(a, b)` | AND (intersection) |
| `Bitset.bitwise_or(a, b)` | OR (union) |
| `Bitset.bitwise_xor(a, b)` | XOR (symmetric difference) |
| `Bitset.flip_all(bs)` | NOT (complement) |
| `Bitset.difference(a, b)` | AND-NOT (set difference) |

### Query Operations

| Function | Description |
|---|---|
| `Bitset.popcount(bs)` | Count set bits |
| `Bitset.size(bs)` | Logical length |
| `Bitset.capacity(bs)` | Allocated bits (multiple of 64) |
| `Bitset.any?(bs)` | Any bit set? |
| `Bitset.all?(bs)` | All bits set? (vacuous truth for empty) |
| `Bitset.none?(bs)` | No bits set? |

### Conversion

| Function | Description |
|---|---|
| `Bitset.set_bits(bs)` | List of set bit indices |
| `Bitset.to_integer(bs)` | Convert to integer |
| `Bitset.to_binary_str(bs)` | Convert to binary string |
| `Bitset.equal?(a, b)` | Structural equality |

## Design Notes

- **Immutable**: All operations return new bitsets (standard Elixir convention)
- **LSB-first**: Bit 0 is the least significant bit of word 0
- **Auto-growth**: `set/2` and `toggle/2` grow the bitset with ArrayList-style capacity doubling
- **Clean-trailing-bits**: Bits beyond `len` are always zero, ensuring correct popcount/equality
- **Reserved word avoidance**: Uses `bitwise_and`, `bitwise_or`, `flip_all` instead of Elixir reserved words `and`, `or`, `not`

## Running Tests

```bash
mix test --cover
```
