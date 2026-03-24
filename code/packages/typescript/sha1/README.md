# @coding-adventures/sha1 (TypeScript)

SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch in TypeScript.

## What It Does

SHA-1 takes any sequence of bytes and produces a fixed-size 20-byte (160-bit) digest.
The same input always yields the same digest. Change one bit of input and the entire
digest changes — the avalanche effect. This package implements SHA-1 from scratch,
without using the Web Crypto API, so every step of the algorithm is visible.

## How It Fits in the Stack

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
monorepo. SHA-1 is a prerequisite for the UUID v5 package.

## JavaScript Caveat

Bitwise operators in JavaScript return signed 32-bit integers. We use `>>> 0` throughout
to coerce results to unsigned 32-bit, which is what SHA-1 requires.

## Usage

```typescript
import { sha1, sha1Hex, toHex, SHA1Hasher } from "@coding-adventures/sha1";

const enc = new TextEncoder();

// One-shot
const digest = sha1(enc.encode("abc"));          // Uint8Array, length 20
const hex = sha1Hex(enc.encode("abc"));          // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Streaming
const h = new SHA1Hasher();
h.update(enc.encode("ab")).update(enc.encode("c"));
console.log(h.hexDigest());                      // "a9993e364706816aba3e25717850c26c9cd0d89d"

// Copy for prefix hashing
const h2 = h.copy();
```

## FIPS 180-4 Test Vectors

```typescript
assert(sha1Hex(enc.encode("")) === "da39a3ee5e6b4b0d3255bfef95601890afd80709");
assert(sha1Hex(enc.encode("abc")) === "a9993e364706816aba3e25717850c26c9cd0d89d");
```

## Development

```bash
npm ci
npx vitest run --coverage
```

Tests: 39 tests, 100% coverage.
