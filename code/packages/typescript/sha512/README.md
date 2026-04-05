# @coding-adventures/sha512 (TypeScript)

SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch in TypeScript.

## What It Does

Computes 64-byte (512-bit) digests using the SHA-2 family algorithm with 64-bit word operations. SHA-512 processes 128-byte blocks through 80 rounds of compression, producing a 128-character hex digest.

## How It Works

SHA-512 is structurally identical to SHA-256 but uses 64-bit words instead of 32-bit. This means wider state (8 x 64-bit), larger blocks (128 bytes vs 64), more rounds (80 vs 64), and different rotation amounts tuned for 64-bit arithmetic.

Since JavaScript lacks native 64-bit integer support, this implementation uses BigInt for all 64-bit operations. BigInt is slower than Number for 32-bit work, but it makes the code clear and correct for educational purposes.

## Usage

```typescript
import { sha512, sha512Hex, SHA512Hasher } from "@coding-adventures/sha512";

const enc = new TextEncoder();

// One-shot hashing
const digest = sha512(enc.encode("hello"));     // Uint8Array (64 bytes)
const hex = sha512Hex(enc.encode("hello"));      // 128-char hex string

// Streaming (for large data)
const h = new SHA512Hasher();
h.update(enc.encode("hello "));
h.update(enc.encode("world"));
h.hexDigest();  // same as sha512Hex("hello world")
```

## API

| Function | Returns | Description |
|----------|---------|-------------|
| `sha512(data)` | `Uint8Array` | 64-byte digest |
| `sha512Hex(data)` | `string` | 128-char lowercase hex |
| `toHex(bytes)` | `string` | Convert any bytes to hex |
| `new SHA512Hasher()` | `SHA512Hasher` | Streaming hasher |
| `.update(data)` | `this` | Feed bytes (chainable) |
| `.digest()` | `Uint8Array` | Get 64-byte digest |
| `.hexDigest()` | `string` | Get 128-char hex |
| `.copy()` | `SHA512Hasher` | Independent copy |

## Dependencies

None. Implemented from scratch.
