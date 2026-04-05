# @coding-adventures/sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in TypeScript.

## What Is SHA-256?

SHA-256 is a member of the SHA-2 family designed by the NSA and published by NIST in 2001. It produces a 256-bit (32-byte) digest and is the workhorse of modern cryptography -- used in TLS, Bitcoin, git, code signing, and password hashing.

Unlike MD5 (broken 2004) and SHA-1 (broken 2017), SHA-256 remains secure with no known practical attacks. The birthday bound is 2^128 operations.

## API

### One-shot Functions

```ts
import { sha256, sha256Hex, toHex } from "@coding-adventures/sha256";

const enc = new TextEncoder();

// Returns 32-byte Uint8Array
const digest = sha256(enc.encode("abc"));

// Returns 64-character hex string
sha256Hex(enc.encode("abc"));
// "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

// Convert any Uint8Array to hex
toHex(digest);
```

### Streaming Hasher

```ts
import { SHA256Hasher } from "@coding-adventures/sha256";

const h = new SHA256Hasher();
h.update(enc.encode("ab"));
h.update(enc.encode("c"));
h.hexDigest(); // same as sha256Hex("abc")

// Branching with copy()
const base = new SHA256Hasher().update(enc.encode("common"));
const h1 = base.copy(); h1.update(enc.encode("A"));
const h2 = base.copy(); h2.update(enc.encode("B"));
```

## Algorithm

SHA-256 follows the Merkle-Damgard construction:

1. **Pad** the message to a multiple of 64 bytes
2. **Split** into 64-byte blocks
3. **Compress** each block into an 8-word (256-bit) state using 64 rounds
4. **Output** the final state as 32 bytes

Key differences from SHA-1:
- 8 state words (vs 5), 64 rounds (vs 80), 64 unique constants (vs 4)
- Non-linear message schedule using sigma0 and sigma1 rotation functions
- Ch and Maj auxiliary functions (no parity rounds)

## Dependencies

None. Pure TypeScript, no runtime dependencies.

## How It Fits

Part of the `coding-adventures` monorepo hash function family (MD5, SHA-1, SHA-256).
