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

## Fixed-memory AES-128-GCM prerequisite

`ghash_multiply_fixed` and `ghash_fixed` provide the repository-owned GHASH
piece used by the reviewed fixed-memory AES-128-GCM boundary. They use caller-owned
fixed output storage, allocate no memory, run the field multiplication with a
fixed 128-round schedule and mask secret-derived decisions, and wipe the
working multiplier, product, accumulator, and padded block on return. GHASH
input lengths are validated before output changes.

The input-bit mask also has wipe-on-drop ownership. A best-effort compiler
barrier hides its zero/all-ones range so the tested optimizer does not skip
the XOR work based on a secret bit. Rust's `black_box` offers no cryptographic
guarantee: the strict optimized-IR gate remains required for each compiler and
target, and passing it is evidence only for the generated code inspected.

`aes128_gcm_encrypt_fixed` and `aes128_gcm_decrypt_fixed` compose GHASH with
the repository-owned fixed-memory AES-128 block boundary for the RFC 5116
96-bit nonce profile. They validate exact caller-output length, encoded length
domains, and the complete 32-bit payload counter range before mutation.
Decryption authenticates the complete 128-bit tag with a release-codegen-gated
fixed schedule before writing plaintext, so rejection leaves caller output
unchanged. Hash subkeys, counters, keystream, GHASH results, encrypted `J0`,
and recomputed tags live in wipe-on-drop fixed storage. The original key,
nonce, inputs, and successful outputs remain caller-owned.

The allocation-returning legacy GCM API remains an educational compatibility
surface. The fixed boundary performs pure in-process cryptography with no
credential access, edit, signing operation, provider action, audit or trace
emission, I/O, or external effect. It does not select a TLS suite, own a
traffic key, reserve nonces, enforce per-key record limits, frame or protect
records, process a handshake, validate certificates, enforce revocation,
authorize HTTP, or provide OAuth transport. A later TLS record
composition must bind this primitive to `TLS_AES_128_GCM_SHA256` and the
existing selected-suite record state. Concrete HTTPS remains unavailable.

## Part of coding-adventures

An educational computing stack built from logic gates through compilers.
