# bitset-wasm

WebAssembly bindings for the [bitset](../../rust/bitset/) library, exposing a compact boolean array packed into 64-bit words for use in browsers and Deno.

## What This Does

This package wraps the pure-Rust `bitset` crate with `wasm-bindgen` so it can be called from JavaScript/TypeScript in browser or server-side WASM runtimes. Every method on the Rust `Bitset` struct is available as a camelCase method on the `WasmBitset` JS class.

## How It Fits in the Stack

```
┌──────────────────────────┐
│   Browser / Deno / Node  │  <-- JavaScript consumer
├──────────────────────────┤
│   bitset-wasm (this)     │  <-- wasm-bindgen wrapper (cdylib)
├──────────────────────────┤
│   bitset (Rust)          │  <-- core logic, zero deps
└──────────────────────────┘
```

The core `bitset` crate has zero dependencies and contains all the logic. This crate adds only `wasm-bindgen` for the JS interop glue.

## Building

```bash
# Native unit tests (no WASM tooling needed):
cargo test

# Build WASM for browsers:
wasm-pack build --target web

# Build WASM for Node.js/Deno:
wasm-pack build --target nodejs
```

## Usage (JavaScript)

```javascript
import init, { WasmBitset } from './bitset_wasm.js';

await init();  // initialize WASM module

// Create a bitset
const bs = new WasmBitset(64);
bs.set(0);
bs.set(5);
bs.set(10);

bs.test(5);       // true
bs.popcount();    // 3
bs.toBinaryStr(); // "10000100001"

// Bitwise operations return new bitsets
const a = WasmBitset.fromBinaryStr("1100");
const b = WasmBitset.fromBinaryStr("1010");

a.and(b).toBinaryStr();    // "1000"
a.or(b).toBinaryStr();     // "1110"
a.xor(b).toBinaryStr();    // "110"
a.not().toBinaryStr();     // "11"
a.andNot(b).toBinaryStr(); // "100"

// Iteration
const bits = bs.iterSetBits();  // [0, 5, 10]

// Conversion
const n = bs.toInteger();  // number or null
```

## API Reference

### Constructors

| Method | Description |
|--------|-------------|
| `new WasmBitset(size)` | Create a bitset with `size` zero bits |
| `WasmBitset.fromInteger(n)` | Create from a non-negative integer |
| `WasmBitset.fromBinaryStr(s)` | Create from `"1010"` style string |

### Bit Manipulation

| Method | Description |
|--------|-------------|
| `set(i)` | Set bit `i` to 1 (grows if needed) |
| `clear(i)` | Clear bit `i` to 0 |
| `test(i)` | Returns true if bit `i` is 1 |
| `toggle(i)` | Flip bit `i` |

### Bitwise Operations (return new bitset)

| Method | Description |
|--------|-------------|
| `and(other)` | Bitwise AND |
| `or(other)` | Bitwise OR |
| `xor(other)` | Bitwise XOR |
| `not()` | Bitwise NOT |
| `andNot(other)` | AND-NOT (bit clear) |

### Queries

| Method | Description |
|--------|-------------|
| `popcount()` | Count of set bits |
| `len()` | Logical length (addressable bits) |
| `capacity()` | Allocated capacity |
| `any()` | True if any bit is set |
| `all()` | True if all bits are set |
| `none()` | True if no bits are set |
| `isEmpty()` | True if length is 0 |

### Conversion

| Method | Description |
|--------|-------------|
| `iterSetBits()` | JS array of set bit indices |
| `toInteger()` | Number or null (if > 64 bits) |
| `toBinaryStr()` | Binary string like `"1010"` |
