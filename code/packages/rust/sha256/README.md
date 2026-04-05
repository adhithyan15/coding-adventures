# coding_adventures_sha256

SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch in Rust.

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

// Branching
let mut base = Sha256Hasher::new();
base.update(b"common");
let mut h1 = base.clone_hasher();
h1.update(b"A");
let mut h2 = base.clone_hasher();
h2.update(b"B");
```

## Algorithm

SHA-256 follows the Merkle-Damgard construction with 8 x 32-bit state words, 64 rounds per block, and a non-linear message schedule using sigma0/sigma1 rotation functions.

## Dependencies

None. Pure Rust, no external crates.

## How It Fits

Part of the `coding-adventures` monorepo hash function family (MD5, SHA-1, SHA-256).
