# @coding-adventures/wasm-leb128

LEB128 (Little-Endian Base 128) variable-length integer encoding for the
WebAssembly binary format. Part of the
[coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo — a ground-up implementation of the computing stack from transistors
to operating systems.

## What is LEB128?

WebAssembly uses LEB128 encoding for every integer value in its binary
(`.wasm`) format: function indices, type indices, memory sizes, instruction
immediates, section lengths, and more.

The idea: instead of always storing a 32-bit integer in 4 bytes, use a
variable-length encoding where small values take 1 byte and large values take
up to 5 bytes. Each byte contributes 7 bits of data; the 8th (high) bit is
a continuation flag:

```
bit 7 = 1  →  more bytes follow
bit 7 = 0  →  this is the last byte
```

### Example: encoding 624485

```
624485 = 0b0001_0011_0000_0111_0110_0101

7-bit groups (little-endian):
  bits 0–6:   0b110_0101 = 0x65 → byte 0: 0xE5  (continuation bit set)
  bits 7–13:  0b000_1110 = 0x0E → byte 1: 0x8E  (continuation bit set)
  bits 14–19: 0b010_0110 = 0x26 → byte 2: 0x26  (last byte, no continuation)

Encoded: [0xE5, 0x8E, 0x26]
```

## Installation

```bash
npm install @coding-adventures/wasm-leb128
```

## API

```typescript
import {
  decodeUnsigned,
  decodeSigned,
  encodeUnsigned,
  encodeSigned,
  LEB128Error,
} from "@coding-adventures/wasm-leb128";
```

### `decodeUnsigned(data, offset?)`

Decode a ULEB128-encoded unsigned integer.

```typescript
const [value, bytesConsumed] = decodeUnsigned(new Uint8Array([0xE5, 0x8E, 0x26]));
// value = 624485, bytesConsumed = 3

// With offset — read from position 1:
const [v, n] = decodeUnsigned(new Uint8Array([0xAA, 0xE5, 0x8E, 0x26]), 1);
// v = 624485, n = 3
```

### `decodeSigned(data, offset?)`

Decode a SLEB128-encoded signed integer (two's complement sign extension).

```typescript
const [value, bytesConsumed] = decodeSigned(new Uint8Array([0x7E]));
// value = -2, bytesConsumed = 1
```

### `encodeUnsigned(value)`

Encode a non-negative integer (u32 range) as ULEB128.

```typescript
encodeUnsigned(624485);  // Uint8Array [0xE5, 0x8E, 0x26]
encodeUnsigned(0);       // Uint8Array [0x00]
```

### `encodeSigned(value)`

Encode a signed integer (i32 range) as SLEB128.

```typescript
encodeSigned(-2);          // Uint8Array [0x7E]
encodeSigned(2147483647);  // Uint8Array [0xFF, 0xFF, 0xFF, 0xFF, 0x07]
```

### `LEB128Error`

Thrown when decoding encounters:
- An unterminated sequence (all bytes have continuation bit = 1)
- A value exceeding the 32-bit range (more than 5 bytes)

```typescript
try {
  decodeUnsigned(new Uint8Array([0x80, 0x80])); // unterminated!
} catch (e) {
  if (e instanceof LEB128Error) {
    console.error("Invalid LEB128:", e.message);
  }
}
```

## Limitations

This package handles **u32 and i32 ranges only** (values encoded in at most 5
LEB128 bytes). WebAssembly also uses i64/u64 values in some contexts (e.g.,
`i64.const` immediates). Those require JavaScript's `BigInt` and are outside
the scope of this package.

## Where LEB128 fits in the stack

```
WebAssembly Binary Format (.wasm)
  └─ Sections: type, import, function, code, data, ...
       └─ All integer values encoded as LEB128
            └─ This package ← you are here
```

## Development

```bash
npm install
npx vitest run --coverage
```

```bash
# Run tests
bash BUILD
```
