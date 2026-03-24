# bitset

A compact data structure that packs boolean values into 64-bit words for space-efficient storage and fast bulk bitwise operations.

## What is a Bitset?

A bitset stores a sequence of bits (0s and 1s) packed into `u64` words. Instead of using one byte per boolean, a bitset stores 64 booleans per word. This gives you:

- **8x less memory** than `Vec<bool>` (64x less than Python's `list[bool]`)
- **64x faster bulk operations** -- AND, OR, XOR process 64 bits per CPU instruction
- **Efficient iteration** -- skip entire zero words, find set bits via hardware `trailing_zeros`

Bitsets appear in Bloom filters, register allocators, graph algorithms (visited sets), database bitmap indexes, filesystem free-block bitmaps, and garbage collectors.

## Layer Position

The bitset is a standalone foundation package with zero dependencies. It sits beneath higher-level data structures like Bloom filters, bitmap indexes, and adjacency matrices.

```
         Future Consumers
    ┌──────────────┬──────────────┬──────────────────┐
    │ Bloom Filter  │ Bitmap Index │ Adjacency Matrix │
    └──────┬───────┴──────┬───────┴────────┬─────────┘
           │              │                │
           ▼              ▼                ▼
    ┌────────────────────────────────────────────────┐
    │                    Bitset                       │
    │  Foundation layer. No dependencies.              │
    └─────────────────────────────────────────────────┘
```

## Usage

### Creating Bitsets

```rust
use bitset::Bitset;

// Create with a specific size (all zeros)
let bs = Bitset::new(100);

// From an integer (bit 0 = LSB)
let bs = Bitset::from_integer(42);  // binary: 101010

// From a binary string (leftmost = highest bit)
let bs = Bitset::from_binary_str("1010").unwrap();  // bits 1,3 set
```

### Single-Bit Operations

```rust
let mut bs = Bitset::new(100);
bs.set(42);          // set bit 42 to 1
bs.clear(42);        // set bit 42 to 0
bs.toggle(42);       // flip bit 42
let is_set = bs.test(42);  // check if bit 42 is 1

// set() and toggle() auto-grow the bitset
bs.set(500);         // grows capacity to fit bit 500
```

### Bulk Operations

All bulk operations return a new bitset without modifying the operands:

```rust
let a = Bitset::from_integer(0b1100);
let b = Bitset::from_integer(0b1010);

let intersection = a.and(&b);   // 0b1000 -- both bits set
let union = a.or(&b);           // 0b1110 -- either bit set
let sym_diff = a.xor(&b);       // 0b0110 -- bits that differ
let complement = a.not();        // flips all bits within len
let difference = a.and_not(&b);  // 0b0100 -- in a but not b
```

Operator overloading is supported:

```rust
let c = &a & &b;  // same as a.and(&b)
let c = &a | &b;  // same as a.or(&b)
let c = &a ^ &b;  // same as a.xor(&b)
let c = !&a;       // same as a.not()
```

### Counting and Queries

```rust
let bs = Bitset::from_integer(0b10110);

bs.popcount();   // 3 (number of set bits)
bs.len();        // 5 (logical size)
bs.capacity();   // 64 (allocated bits, multiple of 64)
bs.any();        // true (at least one bit set)
bs.all();        // false (not all bits set)
bs.none();       // false (some bits are set)
```

### Iteration

Efficiently iterate over set bit indices using the trailing-zeros trick:

```rust
let bs = Bitset::from_integer(0b10100101);
let bits: Vec<usize> = bs.iter_set_bits().collect();
assert_eq!(bits, vec![0, 2, 5, 7]);
```

### Conversion

```rust
let bs = Bitset::from_integer(42);

bs.to_integer();     // Some(42)
bs.to_binary_str();  // "101010"
format!("{}", bs);   // "Bitset(101010)"
```

## API Reference

### Constructors

| Method | Description |
|--------|-------------|
| `Bitset::new(size)` | Create with all zeros, capacity rounded up to multiple of 64 |
| `Bitset::from_integer(u128)` | Create from integer value (bit 0 = LSB) |
| `Bitset::from_binary_str(&str)` | Create from `"0"`/`"1"` string (leftmost = MSB) |

### Single-Bit Operations

| Method | Auto-grows? | Description |
|--------|-------------|-------------|
| `set(i)` | Yes | Set bit i to 1 |
| `clear(i)` | No | Set bit i to 0 (no-op if i >= len) |
| `test(i)` | No | Returns true if bit i is 1 (false if i >= len) |
| `toggle(i)` | Yes | Flip bit i |

### Bulk Operations

| Method | Description |
|--------|-------------|
| `and(&other)` | Intersection (both set) |
| `or(&other)` | Union (either set) |
| `xor(&other)` | Symmetric difference (bits differ) |
| `not()` | Complement (flip all within len) |
| `and_not(&other)` | Set difference (in self but not other) |

### Counting

| Method | Description |
|--------|-------------|
| `popcount()` | Number of 1-bits |
| `len()` | Logical size |
| `capacity()` | Allocated bits (multiple of 64) |
| `any()` | At least one bit set? |
| `all()` | All bits within len set? (vacuous truth for len=0) |
| `none()` | No bits set? |

### Iteration

| Method | Description |
|--------|-------------|
| `iter_set_bits()` | Iterator over indices of set bits (ascending) |

### Conversion

| Method | Description |
|--------|-------------|
| `to_integer()` | `Option<u64>` (None if > 64 bits needed) |
| `to_binary_str()` | String of '0'/'1' (MSB on left) |
| `Display` trait | `"Bitset(101)"` format |

## Internal Design

- **Word type**: `u64` (64 bits per word)
- **Bit ordering**: LSB-first (bit 0 = least significant bit of word 0)
- **Growth**: ArrayList-style doubling (minimum 64 bits)
- **Clean-trailing-bits invariant**: bits beyond `len` are always zero
- **Zero dependencies**: standalone foundation package

## Building and Testing

```bash
cargo test -p bitset -- --nocapture
```
