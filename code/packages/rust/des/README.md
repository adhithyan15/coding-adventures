# coding_adventures_des (Rust)

DES (Data Encryption Standard) and Triple DES (3DES/TDEA) block cipher
implemented from scratch in Rust, following FIPS 46-3 and NIST SP 800-67.

**Educational use only.** DES is cryptographically broken (56-bit keys exhausted
in under 24 hours). 3DES was deprecated by NIST in 2017 and disallowed in 2023
(SWEET32 attack). Use AES for any real application.

## What This Package Teaches

- **Feistel networks** — the structural innovation that makes encryption and
  decryption identical circuits (just reverse the subkey order).
- **S-boxes** — the non-linear heart of DES; hardened by the NSA against
  differential cryptanalysis a decade before that attack was published.
- **Key schedules** — how a single 56-bit key expands into 16 round keys via
  PC-1, PC-2, and 28-bit left-rotation registers.
- **Why 56 bits is not enough** — the brute-force math that doomed DES by 1999.

## Algorithm

```
plaintext (8 bytes)
     │
IP (initial permutation)       ← scatters bits for 1970s bus alignment
     │
┌── 16 Feistel rounds ──────────────────────────────────────────────┐
│   L_i = R_{i-1}                                                   │
│   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                             │
│                                                                   │
│   f(R, K):                                                        │
│     E(R)    32→48 bits (expansion, border bits shared)            │
│     XOR K_i 48-bit subkey                                         │
│     S-boxes 8 × (6 bits → 4 bits) = 32 bits out                  │
│     P       32→32 bit permutation                                 │
└───────────────────────────────────────────────────────────────────┘
     │
FP (final permutation = IP⁻¹)
     │
ciphertext (8 bytes)
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
coding_adventures_des = { path = "../des" }
```

```rust
use coding_adventures_des::{encrypt_block, decrypt_block, ecb_encrypt, ecb_decrypt,
                             tdea_encrypt_block, tdea_decrypt_block};

// Single block
let key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];
let plain = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
let ct = encrypt_block(&plain, &key);
assert_eq!(ct, [0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05]);
assert_eq!(decrypt_block(&ct, &key), plain);

// ECB mode (variable-length, PKCS#7 padding)
let ct = ecb_encrypt(b"Hello, World!", &key);
assert_eq!(ecb_decrypt(&ct, &key).unwrap(), b"Hello, World!");

// Triple DES (EDE ordering)
let k1 = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
let k2 = [0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01];
let k3 = [0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23];
let ct = tdea_encrypt_block(&plain, &k1, &k2, &k3);
assert_eq!(tdea_decrypt_block(&ct, &k1, &k2, &k3), plain);
```

## Stack in This Repository

- This package implements `SE01` (symmetric encryption layer 1 — DES).
- The companion `coding_adventures_aes` package implements `SE02` (AES).
- See `code/specs/` for full algorithm specifications.
