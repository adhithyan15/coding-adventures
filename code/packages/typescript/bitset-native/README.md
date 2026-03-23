# @coding-adventures/bitset-native

Native Node.js addon wrapping the Rust `bitset` crate via our zero-dependency `node-bridge` FFI layer. No napi-rs, no napi-sys, no build-time headers -- just raw N-API calls from Rust.

## What it does

Provides a `Bitset` class that packs boolean values into 64-bit machine words for space-efficient storage and lightning-fast bulk bitwise operations. All operations (AND, OR, XOR, NOT) process 64 bits per CPU instruction.

## How it fits in the stack

```
┌─────────────────────────────────┐
│  JavaScript / TypeScript        │  <-- your code
├─────────────────────────────────┤
│  @coding-adventures/bitset-native │  <-- this package (index.js + .node binary)
├─────────────────────────────────┤
│  node-bridge (Rust crate)       │  <-- zero-dep N-API wrapper
├─────────────────────────────────┤
│  bitset (Rust crate)            │  <-- the actual bitset implementation
└─────────────────────────────────┘
```

## Usage

```typescript
import { Bitset } from "@coding-adventures/bitset-native";

// Create a bitset with 100 bits, all zero
const bs = new Bitset(100);

// Set some bits
bs.set(0);
bs.set(42);
bs.set(99);

// Query
bs.test(42);      // true
bs.popcount();     // 3
bs.iterSetBits();  // [0, 42, 99]

// Create from integer (bit 0 = LSB)
const a = new Bitset(0b1100, "integer");  // bits 2 and 3

// Create from binary string (leftmost = MSB)
const b = new Bitset("1010", "binary");   // bits 1 and 3

// Bulk operations return new bitsets
const intersection = a.and(b);
const union = a.or(b);
const difference = a.andNot(b);
const flipped = a.not();

// Conversion
bs.toInteger();     // 42 or null if too large
bs.toBinaryStr();   // "101010..."
```

## Building

Requires Rust toolchain and Node.js:

```bash
npm ci
cargo build --release
cp target/release/bitset_native_node.node .   # Windows: .dll -> .node
npx vitest run
```

Or use the BUILD file with the project's build tool.

## API

See `index.d.ts` for full TypeScript type definitions.

### Construction

| Constructor | Description |
|---|---|
| `new Bitset(size)` | Zero-filled bitset with `size` addressable bits |
| `new Bitset(value, "integer")` | From a non-negative integer |
| `new Bitset(str, "binary")` | From a binary string like `"1010"` |

### Single-bit operations

| Method | Description |
|---|---|
| `set(i)` | Set bit i to 1 (auto-grows) |
| `clear(i)` | Set bit i to 0 (no-op if out of range) |
| `test(i)` | Returns true if bit i is set |
| `toggle(i)` | Flip bit i (auto-grows) |

### Bulk bitwise operations

| Method | Description |
|---|---|
| `and(other)` | Intersection (new bitset) |
| `or(other)` | Union (new bitset) |
| `xor(other)` | Symmetric difference (new bitset) |
| `not()` | Complement (new bitset) |
| `andNot(other)` | Difference (new bitset) |

### Query operations

| Method | Description |
|---|---|
| `popcount()` | Number of set bits |
| `len()` | Logical length in bits |
| `capacity()` | Allocated capacity in bits |
| `any()` | True if any bit is set |
| `all()` | True if all bits are set |
| `none()` | True if no bits are set |
| `isEmpty()` | True if len is 0 |

### Conversion

| Method | Description |
|---|---|
| `iterSetBits()` | Array of set bit indices |
| `toInteger()` | Number or null if too large |
| `toBinaryStr()` | Binary string representation |
