# coding_adventures_aes (Rust)

AES (Advanced Encryption Standard) block cipher implemented from scratch in Rust,
following FIPS 197. Supports AES-128, AES-192, and AES-256.

Uses GF(2^8) arithmetic from the companion `gf256` crate (polynomial 0x11B).

## What This Package Teaches

- **Substitution-Permutation Network (SPN)** — how all 16 bytes of the state are
  transformed each round (vs DES Feistel which only transforms half).
- **GF(2^8) arithmetic** — how the AES S-box (SubBytes) computes the
  multiplicative inverse in GF(2^8) with polynomial x^8 + x^4 + x^3 + x + 1.
- **MixColumns** — GF(2^8) matrix multiplication for diffusion across columns.
- **ShiftRows** — cyclic row shifts that force cross-column diffusion.
- **Key schedule** — how Nk words expand to 4(Nr+1) round words using SubWord,
  RotWord, and round constants (Rcon).

## Architecture

```
plaintext (16 bytes)
     │
AddRoundKey(state, round_key[0])       ← XOR with first key material
     │
┌── Nr-1 full rounds ──────────────────────────────────────────────┐
│   SubBytes   — non-linear S-box substitution (GF(2^8) inverse)   │
│   ShiftRows  — cyclic row shifts (diffusion across columns)       │
│   MixColumns — GF(2^8) matrix multiply (diffusion across rows)   │
│   AddRoundKey — XOR with round key                               │
└───────────────────────────────────────────────────────────────────┘
     │
SubBytes + ShiftRows + AddRoundKey     ← final round (no MixColumns)
     │
ciphertext (16 bytes)
```

## Key Sizes

| Key size  | Nk | Nr | Round keys |
|-----------|----|----|------------|
| 128 bits  |  4 | 10 |     11     |
| 192 bits  |  6 | 12 |     13     |
| 256 bits  |  8 | 14 |     15     |

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
coding_adventures_aes = { path = "../aes" }
```

```rust
use coding_adventures_aes::{encrypt_block, decrypt_block, sbox, inv_sbox};

// AES-128
let key = [
    0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
    0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
];
let plain = [
    0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d,
    0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34,
];
let ct = encrypt_block(&plain, &key).unwrap();
// ct == [0x39, 0x25, 0x84, 0x1d, ...]
assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);

// AES-256
let key256 = vec![0u8; 32]; // 32 bytes = 256-bit key
let ct256 = encrypt_block(&plain, &key256).unwrap();
assert_eq!(decrypt_block(&ct256, &key256).unwrap(), plain);

// Access the S-box
assert_eq!(sbox()[0x00], 0x63); // FIPS 197 Figure 7
assert_eq!(inv_sbox()[0x63], 0x00);
```

## Stack in This Repository

- This package implements `SE02` (symmetric encryption layer 2 — AES).
- Depends on `gf256` for GF(2^8) arithmetic.
- The companion `coding_adventures_des` package implements `SE01` (DES).
- See `code/specs/` for full algorithm specifications.
