# coding_adventures_sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Rust. The core digest and streaming APIs use fixed memory and retain no heap-backed message buffer.

## What Is SHA-256?

SHA-256 is a member of the SHA-2 family designed by the NSA and published by NIST in 2001. It produces a 256-bit (32-byte) digest and is the workhorse of modern cryptography -- used in TLS, Bitcoin, git, code signing, and password hashing.

## API

### One-shot Functions

```rust
use coding_adventures_sha256::{sha256, sha256_hex};

// Returns [u8; 32]
let digest = sha256(b"abc");

// Returns 64-character hex String
let hex = sha256_hex(b"abc");
// "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
```

### Streaming Hasher

```rust
use coding_adventures_sha256::{Sha256Hasher, sha256};

let mut h = Sha256Hasher::new();
h.update(b"ab");
h.update(b"c");
assert_eq!(h.digest(), sha256(b"abc"));

// Security-sensitive callers use the checked operation. It rejects a total
// message whose bit length would not fit FIPS 180-4 before changing state.
h.try_update(b"more data")?;

// Branching
let mut base = Sha256Hasher::new();
base.update(b"common");
let mut h1 = base.clone_hasher();
h1.update(b"A");
let mut h2 = base.clone_hasher();
h2.update(b"B");

# Ok::<(), coding_adventures_sha256::Sha256Error>(())
```

`update()` remains as the original infallible compatibility API.
`try_update()` is the production boundary for callers such as a future HMAC:
it reports cumulative length overflow as a payload-free `Sha256Error`.
Snapshots and clones are non-destructive and fixed-memory. The hasher scrubs
its retained chaining state and partial block on drop; digest snapshots also
scrub temporary state, padding, and compression schedules.

## Algorithm

SHA-256 follows the Merkle-Damgard construction with 8 x 32-bit state words, 64 rounds per block, and a non-linear message schedule using sigma0/sigma1 rotation functions.

## Dependencies

- `coding_adventures_zeroize` (repository-owned secure memory wiping)

No external crates are used.

## How It Fits

Part of the `coding-adventures` monorepo hash function family (MD5, SHA-1,
SHA-256, SHA-384, SHA-512). This package is a digest primitive only. It does
not implement HMAC, HKDF, TLS key scheduling, record protection, certificate
validation, networking, or OAuth transport.
