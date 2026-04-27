# @coding-adventures/ed25519

Ed25519 digital signature algorithm (RFC 8032) implemented from scratch using JavaScript's native BigInt arithmetic.

## What is Ed25519?

Ed25519 is a high-speed, high-security digital signature scheme built on the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over the prime field GF(2^255 - 19). It provides:

- **128-bit security** (equivalent to ~3072-bit RSA)
- **Deterministic signatures** (no random nonce needed)
- **32-byte public keys** and **64-byte signatures**
- **Complete addition formula** (resistant to timing attacks)

## How It Fits in the Stack

This package depends on `@coding-adventures/sha512` for:
- Key derivation (SHA-512 of seed)
- Deterministic nonce generation
- Challenge hash computation

## Usage

```typescript
import { generateKeypair, sign, verify } from "@coding-adventures/ed25519";

// Generate a keypair from a 32-byte seed
const seed = crypto.getRandomValues(new Uint8Array(32));
const { publicKey, secretKey } = generateKeypair(seed);

// Sign a message
const message = new TextEncoder().encode("Hello, world!");
const signature = sign(message, secretKey);

// Verify the signature
const valid = verify(message, signature, publicKey);
console.log(valid); // true
```

## API

### `generateKeypair(seed: Uint8Array): { publicKey: Uint8Array, secretKey: Uint8Array }`

Generate a keypair from a 32-byte seed. Returns a 32-byte public key and 64-byte secret key.

### `sign(message: Uint8Array, secretKey: Uint8Array): Uint8Array`

Sign a message with the secret key. Returns a 64-byte deterministic signature.

### `verify(message: Uint8Array, signature: Uint8Array, publicKey: Uint8Array): boolean`

Verify a signature against a message and public key. Returns true if valid.

## Test Vectors

Tested against all RFC 8032 Section 7.1 test vectors.
