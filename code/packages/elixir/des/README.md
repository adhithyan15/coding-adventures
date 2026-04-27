# coding_adventures_des (Elixir)

DES and Triple DES (TDEA) block cipher — FIPS 46-3 / SP 800-67.

**Warning:** DES is cryptographically broken. The 56-bit key can be exhausted
in under 24 hours on consumer hardware. This package is for **education only**.

## What It Is

DES (Data Encryption Standard) was the world's first openly standardized
encryption algorithm, published by NIST in 1977. It uses a **Feistel network**
structure: 16 rounds where each round applies a function f to the right half
and XORs it into the left half. The elegance of a Feistel network is that
**decryption is identical to encryption** — just apply the subkeys in reverse
order. No inverse operations needed.

## How It Fits in the Stack

- **Depends on:** nothing (pure Elixir)
- **Depended on by:** future block cipher mode packages (CBC, CTR, GCM)
- **Related:** `coding_adventures_aes` — the modern replacement for DES

## Usage

```elixir
alias CodingAdventures.Des

key   = <<0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1>>
plain = <<0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF>>

# Encrypt a single block
ct = Des.des_encrypt_block(plain, key)
# => <<0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05>>

# Decrypt
Des.des_decrypt_block(ct, key)
# => <<0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF>>

# ECB mode (variable length, PKCS#7 padding)
ct2 = Des.des_ecb_encrypt("Hello, World!", key)
Des.des_ecb_decrypt(ct2, key)
# => "Hello, World!"

# Triple DES (TDEA EDE)
k1 = <<0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF>>
k2 = <<0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01>>
k3 = <<0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23>>
Des.tdea_encrypt_block(plain, k1, k2, k3)
```

## Running Tests

```bash
mix deps.get && mix test --cover
```
