# coding_adventures_aes

AES (Advanced Encryption Standard) block cipher — FIPS 197. Supports AES-128, AES-192, and AES-256.

## What Is AES?

AES is the most widely deployed symmetric encryption algorithm in the world. Published by NIST in 2001 as FIPS 197, it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption, and virtually every secure protocol.

Unlike DES (Feistel network), AES is a Substitution-Permutation Network (SPN). All 16 bytes of the state are transformed on every round, providing faster diffusion.

## Usage

```ruby
require 'coding_adventures_aes'

# AES-128
key   = ["2b7e151628aed2a6abf7158809cf4f3c"].pack("H*")
plain = ["3243f6a8885a308d313198a2e0370734"].pack("H*")

cipher = CodingAdventures::Aes.aes_encrypt_block(plain, key)
puts cipher.unpack1("H*")  # → "3925841d02dc09fbdc118597196a0b32"

recovered = CodingAdventures::Aes.aes_decrypt_block(cipher, key)
# recovered == plain

# AES-256
key256 = ["603deb1015ca71be2b73aef0857d7781" \
          "1f352c073b6108d72d9810a30914dff4"].pack("H*")
cipher256 = CodingAdventures::Aes.aes_encrypt_block(plain, key256)
```

## API

| Method | Description |
|---|---|
| `Aes.aes_encrypt_block(block, key)` | Encrypt one 16-byte block (AES-128/192/256) |
| `Aes.aes_decrypt_block(block, key)` | Decrypt one 16-byte block |
| `Aes.expand_key(key)` | Expand key into round keys |
| `Aes::SBOX` | 256-entry S-box array |
| `Aes::INV_SBOX` | 256-entry inverse S-box array |

## Key Sizes

| Key | Rounds | Round Keys |
|---|---|---|
| 128 bits (16 bytes) | 10 | 11 |
| 192 bits (24 bytes) | 12 | 13 |
| 256 bits (32 bytes) | 14 | 15 |

## Implementation Notes

- Follows FIPS 197 exactly
- GF(2^8) arithmetic uses inline Russian peasant multiplication with polynomial 0x11B
- No external GF(2^8) gem dependency — self-contained
- S-box computed at load time from GF(2^8) inverses + affine transformation
- Decryption uses InvSubBytes, InvShiftRows, InvMixColumns (distinct from encryption)

## References

- FIPS 197: https://csrc.nist.gov/publications/detail/fips/197/final
- FIPS 197 Appendix B: step-by-step AES-128 worked example
- FIPS 197 Appendix C: AES-128/192/256 test vectors
