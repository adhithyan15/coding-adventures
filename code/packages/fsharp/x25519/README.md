# CodingAdventures.X25519.FSharp

A pure F# implementation of X25519 elliptic-curve Diffie-Hellman from RFC
7748. The implementation uses the Montgomery ladder over `2^255 - 19`, masks
and clamps inputs as required by the RFC, and rejects low-order inputs that
produce an all-zero shared secret.

```fsharp
open CodingAdventures.X25519.FSharp

let alicePublic = X25519.generateKeypair alicePrivate
let bobPublic = X25519.generateKeypair bobPrivate
let shared = X25519.x25519 alicePrivate bobPublic
```

The package is dependency-free beyond the .NET standard library. It uses
`BigInteger` for clarity, so the arithmetic is not guaranteed to be
constant-time and should be treated as an educational implementation.
