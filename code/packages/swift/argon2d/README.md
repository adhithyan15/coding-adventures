# Argon2d (Swift)

From-scratch Swift implementation of **Argon2d** (RFC 9106) — the
data-dependent member of the Argon2 memory-hard KDF family.

Depends only on our sibling `Blake2b` package for the underlying hash.

See the spec at [../../specs/KD03-argon2.md](../../specs/KD03-argon2.md).

## Usage

```swift
import Argon2d

let tag = try Argon2d.argon2d(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32,
    key: [], associatedData: []
)

let hex = try Argon2d.argon2dHex(
    password: Array("secret".utf8),
    salt: Array("0123456789abcdef".utf8),
    timeCost: 3, memoryCost: 32, parallelism: 4, tagLength: 32
)
```

## When to pick Argon2d

| Use case                      | Argon2 variant |
|-------------------------------|----------------|
| Password hashing (any server) | **Argon2id**   |
| Side-channel resistance only  | Argon2i        |
| Proof-of-work, no secret input | **Argon2d**   |

Argon2d chooses every reference block based on the password-derived state,
so memory-access timing **is** a side channel. Don't feed Argon2d data that
must be kept secret from anyone watching cache timings. For password
hashing in adversarial environments use `Argon2id`.

## Implementation notes

- Swift's `UInt64` `&+`, `&*`, `>>`, `<<`, `^` are used directly — no
  masking, no unsafe, no `Foundation`-heavy dependencies beyond
  `String(format:)` for hex.
- The memory matrix is modelled as `[[[UInt64]]]` (`lanes x columns x
  block-words`). Argon2's rule "every block is 1 KiB of u64s" maps onto
  this trivially.
- Tag comparison is NOT provided here — callers MUST compare tags with
  a constant-time helper (see our `ct-compare` crate in Rust; equivalent
  helpers exist in the other languages).

## Trust boundary

Argon2 is designed to burn CPU and RAM on purpose. Callers control the
DoS boundary via `timeCost`, `memoryCost`, and `parallelism`. Don't
expose those to untrusted input without bounds.

## Running the tests

```bash
swift test --enable-code-coverage
```

The gold-standard RFC 9106 §5.1 vector is verified along with validation,
determinism, key/AD binding, tag-length boundary, and parallelism tests.
