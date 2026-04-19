# @coding-adventures/blake2b

A from-scratch TypeScript implementation of the **BLAKE2b** cryptographic
hash function (RFC 7693).  No third-party runtime dependencies.

## What is BLAKE2b?

BLAKE2b is a modern hash function that is:

- Faster than MD5 on 64-bit hardware.
- As secure as SHA-3 against known attacks.
- Variable output length (1..64 bytes).
- Keyed in a single pass (replaces HMAC-SHA-512).
- Parameterized with salt and personalization.

It is the hash underlying libsodium, WireGuard, Noise Protocol, IPFS
content addressing, and -- within this repo -- Argon2.

See the spec at [code/specs/HF06-blake2b.md](../../../specs/HF06-blake2b.md)
for the full algorithm walkthrough.

## Usage

### One-shot

```ts
import { blake2b, blake2bHex } from "@coding-adventures/blake2b";

const enc = new TextEncoder();
blake2bHex(enc.encode("abc"));                    // 128-char hex
blake2b(enc.encode("abc"), { digestSize: 32 });   // Uint8Array(32)
```

### Keyed (MAC)

```ts
const key = enc.encode("shared secret");
blake2b(message, { key, digestSize: 32 });
```

### Streaming

```ts
import { Blake2bHasher } from "@coding-adventures/blake2b";

const h = new Blake2bHasher({ digestSize: 32 });
h.update(enc.encode("partial "));
h.update(enc.encode("payload"));
console.log(h.hexDigest());
```

### Salt and personalization

```ts
const salt = new Uint8Array(16);      // exactly 16 bytes
const personal = new Uint8Array(16);  // exactly 16 bytes
blake2b(data, { key, salt, personal });
```

## Implementation notes

This package uses JavaScript's native `BigInt` for 64-bit arithmetic.  A
pure-`number` two-32-bit-word emulation would be roughly 3x faster but
significantly more code; BigInt keeps the source a one-to-one transliteration
of RFC 7693 for readability.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are intentionally not included -- see the
"Non-Goals" section of the spec.

## Running the tests

```bash
npm install
npx vitest run --coverage
```

Tests cross-validate against fixed known-answer vectors computed from
Python's `hashlib.blake2b`, which wraps the reference implementation.
