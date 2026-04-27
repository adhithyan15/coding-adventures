# X25519

X25519 Elliptic Curve Diffie-Hellman key agreement (RFC 7748), implemented from scratch with no external dependencies.

## What is X25519?

X25519 is the Diffie-Hellman function on Curve25519. It allows two parties to establish a shared secret over an insecure channel. Used in TLS 1.3, SSH, Signal, WireGuard, and many other protocols.

## Usage

```swift
import X25519

// Generate keypairs (use SecRandomCopyBytes for real private keys)
let alicePrivate: [UInt8] = ... // 32 random bytes
let alicePublic = try X25519.x25519Base(scalar: alicePrivate)

let bobPrivate: [UInt8] = ... // 32 random bytes
let bobPublic = try X25519.x25519Base(scalar: bobPrivate)

// Compute shared secret
let aliceShared = try X25519.x25519(scalar: alicePrivate, u: bobPublic)
let bobShared = try X25519.x25519(scalar: bobPrivate, u: alicePublic)
// aliceShared == bobShared
```

## API

- `X25519.x25519(scalar:u:)` -- scalar multiplication on Curve25519
- `X25519.x25519Base(scalar:)` -- multiply by base point (u=9)
- `X25519.generateKeypair(privateKey:)` -- alias for x25519Base

All inputs/outputs are `[UInt8]` arrays of 32 bytes in little-endian encoding.

## Implementation details

- Custom multi-precision arithmetic using `[UInt64]` limbs (4 limbs = 256 bits)
- Fast modular reduction using the identity 2^255 = 19 (mod p)
- Montgomery ladder for scalar multiplication
- Scalar clamping per RFC 7748
- Fermat's little theorem for modular inversion

## Part of coding-adventures

This package is part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures) educational computing stack.
