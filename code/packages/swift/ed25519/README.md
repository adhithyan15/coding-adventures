# Ed25519 (Swift)

Ed25519 digital signatures (RFC 8032) implemented from scratch in Swift with custom multi-precision arithmetic.

## What Is Ed25519?

Ed25519 is a high-speed, high-security digital signature scheme operating on the twisted Edwards curve `-x^2 + y^2 = 1 + d*x^2*y^2` over `GF(2^255 - 19)`.

Key properties:
- 128-bit security level (equivalent to ~3072-bit RSA)
- Deterministic signatures (no random nonce needed)
- 32-byte public keys, 64-byte signatures
- Complete addition formula (resistant to timing attacks)

## Usage

```swift
import Ed25519

// Generate a keypair from a 32-byte seed
let seed: Data = ... // 32 bytes of randomness
let keypair = generateKeypair(seed: seed)

// Sign a message
let message = Data("Hello, world!".utf8)
let signature = ed25519Sign(message: message, secretKey: keypair.secretKey)

// Verify a signature
let isValid = ed25519Verify(message: message, signature: signature, publicKey: keypair.publicKey)
```

## How It Works

1. **Key generation**: SHA-512 hashes the seed, the first half is clamped to produce a scalar `a`, and the public key is `A = a * B` where `B` is the curve's base point.

2. **Signing**: A deterministic nonce `r` is derived from the secret key and message via SHA-512. The signature `(R, S)` is computed where `R = r*B` and `S = r + H(R||A||M)*a mod L`.

3. **Verification**: Check that `S*B == R + H(R||A||M)*A`.

## Implementation Notes

Swift lacks a built-in arbitrary-precision integer type, so this package implements custom multi-precision arithmetic using `[UInt64]` limb arrays. This is "schoolbook" arithmetic -- correct and clear, though not optimized for speed.

## Dependencies

- SHA-512 (`sha512` package in this monorepo)

## Part of coding-adventures

An educational computing stack built from logic gates up through interpreters and compilers.
