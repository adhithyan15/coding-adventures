# CodingAdventures.X25519

A pure C# implementation of X25519 elliptic-curve Diffie-Hellman from RFC
7748. The implementation uses the Montgomery ladder over `2^255 - 19`, masks
and clamps inputs as required by the RFC, and rejects low-order inputs that
produce an all-zero shared secret.

```csharp
using CodingAdventures.X25519;

byte[] alicePublic = Curve25519.GenerateKeypair(alicePrivate);
byte[] bobPublic = Curve25519.GenerateKeypair(bobPrivate);
byte[] shared = Curve25519.Compute(alicePrivate, bobPublic);
```

The package is dependency-free beyond the .NET standard library. It uses
`BigInteger` for clarity, so the arithmetic is not guaranteed to be
constant-time and should be treated as an educational implementation.
