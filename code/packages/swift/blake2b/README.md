# Blake2b (Swift)

A from-scratch Swift implementation of the **BLAKE2b** cryptographic
hash function (RFC 7693).  No external dependencies.

See the spec at [../../specs/HF06-blake2b.md](../../specs/HF06-blake2b.md)
for the full walk-through.

## Usage

```swift
import Blake2b

// One-shot
let digest = try Blake2b.hashHex(Array("abc".utf8))
let bytes  = try Blake2b.hash(Array("abc".utf8), options: .init(digestSize: 32))

// Keyed (MAC)
let tag = try Blake2b.hash(
    message,
    options: .init(digestSize: 32, key: Array("shared secret".utf8))
)

// Streaming
var h = try Blake2b.Hasher(options: .init(digestSize: 32))
h.update(Array("partial ".utf8))
h.update(Array("payload".utf8))
let out = h.hexDigest()

// Salt + personal (each exactly 16 bytes, or absent)
let salt: [UInt8] = Array(repeating: 0, count: 16)
let personal: [UInt8] = Array(repeating: 0, count: 16)
_ = try Blake2b.hash(data, options: .init(salt: salt, personal: personal))
```

## Implementation notes

Swift has native `UInt64` with `&+` wrapping add; that is used throughout
for the ARX mixing, so the source reads almost exactly like the RFC.
There is no `unsafe`.

The 128-bit byte counter in the spec is modelled by a small
`UInt128Emulated` struct with wrap-on-overflow add so the spec's
reserved-but-usually-zero high word is represented faithfully.

`Hasher` is a value type (`struct`), so `copy()` is literally a
structural copy — the canonical cloning test in the KAT suite passes
trivially.

## Scope

Sequential mode only.  Tree hashing, BLAKE2s, BLAKE2bp, BLAKE2sp,
BLAKE2Xb, and BLAKE3 are out of scope.

## Running the tests

```bash
swift test --enable-code-coverage
```

Tests cross-validate against fixed known-answer vectors precomputed from
Python's `hashlib.blake2b`.  The same KAT table is mirrored across every
language implementation in the monorepo.
