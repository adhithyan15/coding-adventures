# coding_adventures_aes (Elixir)

AES (Advanced Encryption Standard) block cipher — FIPS 197.

Supports AES-128 (16-byte key), AES-192 (24-byte key), and AES-256 (32-byte key).

## What It Is

AES is the most widely deployed symmetric encryption algorithm in the world.
It replaced DES in 2001. Unlike DES's Feistel network (which transforms only
half the state per round), AES is a **Substitution-Permutation Network (SPN)**
that transforms all 16 bytes every round:

- **SubBytes** — non-linear GF(2^8) inverse + affine transform
- **ShiftRows** — cyclic left shifts provide column diffusion
- **MixColumns** — GF(2^8) matrix multiply provides row diffusion
- **AddRoundKey** — XOR with round key

## How It Fits in the Stack

- **Depends on:** nothing (GF arithmetic is inline)
- **Related:** `coding_adventures_des` — the historical predecessor
- **Future:** block cipher modes (CBC, GCM, CTR) will build on this

## Usage

```elixir
alias CodingAdventures.Aes

# AES-128
key   = Base.decode16!("2b7e151628aed2a6abf7158809cf4f3c", case: :lower)
plain = Base.decode16!("3243f6a8885a308d313198a2e0370734", case: :lower)

ct = Aes.aes_encrypt_block(plain, key)
# => <<0x39, 0x25, 0x84, 0x1d, ...>>

Aes.aes_decrypt_block(ct, key) == plain  # => true

# AES-256
key256 = Base.decode16!("603deb1015ca71be2b73aef0857d7781" <>
                        "1f352c073b6108d72d9810a30914dff4", case: :lower)
```

## Running Tests

```bash
mix deps.get && mix test --cover
```
