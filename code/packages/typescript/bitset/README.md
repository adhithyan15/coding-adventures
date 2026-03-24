# @coding-adventures/bitset

A compact bitset data structure that packs boolean values into 32-bit words
using `Uint32Array`. Provides O(n/32) bulk bitwise operations (AND, OR, XOR,
NOT), efficient iteration over set bits using trailing-zero-count, and
ArrayList-style automatic growth.

## Why Uint32Array?

JavaScript has no native 64-bit integer type. `Number` is a 64-bit float that
can only represent integers exactly up to 2^53. `BigInt` exists but is slow and
cannot be used in typed arrays. `Uint32Array` gives us fixed-size 32-bit
unsigned integers with predictable bitwise behavior.

All formulas use 32 as the word size:
- **Word index**: `i >> 5` (which word contains bit i?)
- **Bit offset**: `i & 31` (which position within that word?)
- **Bitmask**: `1 << (i & 31)` (a mask with only bit i set)

## Installation

```bash
npm install @coding-adventures/bitset
```

## Quick Start

```typescript
import { Bitset } from "@coding-adventures/bitset";

// Create a bitset and set some bits
const bs = new Bitset(100);
bs.set(0);
bs.set(42);
bs.set(99);

console.log(bs.popcount());          // 3
console.log([...bs.iterSetBits()]);  // [0, 42, 99]

// Bulk operations return new bitsets
const other = Bitset.fromInteger(42);
const intersection = bs.and(other);
```

## API

### Constructors

- `new Bitset(size)` -- Create a bitset with all bits initially zero
- `Bitset.fromInteger(value)` -- Create from a non-negative integer
- `Bitset.fromBinaryStr(s)` -- Create from a string of '0' and '1' characters

### Single-Bit Operations

- `set(i)` -- Set bit i to 1 (auto-grows if needed)
- `clear(i)` -- Set bit i to 0 (no-op if beyond len)
- `test(i)` -- Test whether bit i is set
- `toggle(i)` -- Flip bit i (auto-grows if needed)

### Bulk Bitwise Operations

All return a new bitset without modifying the operands:

- `and(other)` -- Intersection
- `or(other)` -- Union
- `xor(other)` -- Symmetric difference
- `not()` -- Complement (flips bits within len)
- `andNot(other)` -- Set difference (bits in this but not in other)

### Counting and Queries

- `popcount()` -- Count set bits
- `size` -- Logical length (getter)
- `capacity` -- Allocated bits (getter, always multiple of 32)
- `any()` -- At least one bit set?
- `all()` -- All bits set? (vacuous truth for empty)
- `none()` -- No bits set?
- `isEmpty()` -- Zero length?

### Iteration

- `*iterSetBits()` -- Generator yielding indices of set bits in ascending order

### Conversion

- `toInteger()` -- Convert to number (throws if > 2^53-1)
- `toBinaryStr()` -- Convert to binary string (MSB on left)
- `toString()` -- Human-readable: `"Bitset(101)"`

### Equality

- `equals(other)` -- Same len and same bits?

## Layer Position

The bitset is a standalone foundation package with no dependencies. It sits
beneath higher-level data structures like Bloom filters, bitmap indexes, and
adjacency matrices.

## License

MIT
