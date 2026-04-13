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

## Part of coding-adventures

An educational computing stack built from logic gates through compilers.
