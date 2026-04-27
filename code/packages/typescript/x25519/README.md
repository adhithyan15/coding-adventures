# @coding-adventures/x25519

X25519 Elliptic Curve Diffie-Hellman key agreement (RFC 7748), implemented from scratch with no external dependencies.

## What is X25519?

X25519 is the Diffie-Hellman function on Curve25519. It allows two parties (Alice and Bob) to establish a shared secret over an insecure channel. It is used in TLS 1.3, SSH, Signal, WireGuard, and many other protocols.

## How it works

1. Alice generates a random 32-byte private key and computes her public key: `publicA = x25519Base(privateA)`
2. Bob does the same: `publicB = x25519Base(privateB)`
3. They exchange public keys over the network
4. Alice computes: `shared = x25519(privateA, publicB)`
5. Bob computes: `shared = x25519(privateB, publicA)`
6. Both arrive at the same 32-byte shared secret

## Usage

```typescript
import { x25519, x25519Base, generateKeypair } from "@coding-adventures/x25519";

// Generate keypairs (in practice, use crypto.getRandomValues for private keys)
const alicePrivate = new Uint8Array(32); // fill with random bytes
const alicePublic = generateKeypair(alicePrivate);

const bobPrivate = new Uint8Array(32); // fill with random bytes
const bobPublic = generateKeypair(bobPrivate);

// Compute shared secret
const aliceShared = x25519(alicePrivate, bobPublic);
const bobShared = x25519(bobPrivate, alicePublic);
// aliceShared === bobShared
```

## API

- `x25519(scalar: Uint8Array, u: Uint8Array): Uint8Array` — scalar multiplication on Curve25519
- `x25519Base(scalar: Uint8Array): Uint8Array` — multiply by base point (u=9)
- `generateKeypair(privateKey: Uint8Array): Uint8Array` — alias for x25519Base

All inputs/outputs are 32-byte Uint8Arrays in little-endian encoding.

## Implementation details

- Field arithmetic over GF(2^255-19) using native BigInt
- Montgomery ladder for constant-time scalar multiplication
- Scalar clamping per RFC 7748
- Fermat's little theorem for modular inversion

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
