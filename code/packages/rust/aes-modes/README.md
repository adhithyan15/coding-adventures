# coding_adventures_aes_modes

AES modes of operation — ECB, CBC, CTR, GCM — implemented from scratch in Rust for educational purposes.

## Overview

AES operates on fixed 128-bit blocks. To encrypt arbitrary-length messages, you need a **mode of operation**. This crate implements four modes:

| Mode | Security | Properties |
|------|----------|------------|
| ECB  | **BROKEN** | Identical blocks leak patterns |
| CBC  | Legacy | Padding oracle vulnerable |
| CTR  | Modern | Stream cipher, parallelizable |
| GCM  | Modern + Auth | CTR + GHASH, gold standard |

## Usage

```rust
use coding_adventures_aes_modes::{ecb_encrypt, cbc_encrypt, ctr_encrypt, gcm_encrypt};

let key = hex::decode("2b7e151628aed2a6abf7158809cf4f3c").unwrap();
let plaintext = b"Hello, AES modes!";

// ECB (INSECURE)
let ct = ecb_encrypt(plaintext, &key).unwrap();

// CBC
let iv = hex::decode("000102030405060708090a0b0c0d0e0f").unwrap();
let ct = cbc_encrypt(plaintext, &key, &iv).unwrap();

// CTR
let nonce = hex::decode("f0f1f2f3f4f5f6f7f8f9fafb").unwrap();
let ct = ctr_encrypt(plaintext, &key, &nonce).unwrap();

// GCM
let (ct, tag) = gcm_encrypt(plaintext, &key, &nonce[..12], &[]).unwrap();
```

## Testing

```bash
cargo test -p coding_adventures_aes_modes
```

Uses NIST SP 800-38A and GCM specification test vectors.

## Fixed-memory GHASH prerequisite

`ghash_multiply_fixed` and `ghash_fixed` provide the repository-owned GHASH
piece needed by a future reviewed AES-GCM boundary. They use caller-owned
fixed output storage, allocate no memory, run the field multiplication with a
fixed 128-round schedule and mask secret-derived decisions, and wipe the
working multiplier, product, accumulator, and padded block on return. GHASH
input lengths are validated before output changes.

This is deliberately not a production AES-GCM or TLS implementation. The
legacy GCM API remains an educational compatibility surface, and the current
AES implementation has not been hardened or composed with this primitive into
a reviewed fixed-memory AEAD. Key ownership, nonce construction and reuse
prevention, per-key record limits, record protection, handshake processing,
certificate validation, revocation, networking, HTTP, and OAuth transport all
remain outside this crate.

## Part of coding-adventures

An educational computing stack built from logic gates through compilers.
