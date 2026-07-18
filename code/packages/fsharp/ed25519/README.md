# CodingAdventures.Ed25519

A pure F# implementation of Ed25519 deterministic digital signatures from
RFC 8032. It composes the existing native SHA-512 package with extended
Edwards-coordinate arithmetic over `2^255 - 19`.

```fsharp
open CodingAdventures.Ed25519.FSharp

let publicKey, secretKey = Ed25519.generateKeypair seed
let signature = Ed25519.sign message secretKey
let valid = Ed25519.verify message signature publicKey
```

The 32-byte seed deterministically produces a 32-byte public key and a 64-byte
`seed || publicKey` secret key. Signatures are 64-byte `R || S` values. The
implementation rejects non-canonical scalars and point encodings.

This package uses `BigInteger` for clarity. Its arithmetic is not guaranteed
to be constant-time and should be treated as an educational implementation.
