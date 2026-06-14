# Changelog

All notable changes to `CodingAdventures::ChaCha20Poly1305` will be documented here.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of ChaCha20-Poly1305 AEAD (RFC 8439)
- `chacha20_block` — core 20-round block function with ARX quarter-rounds
- `chacha20_encrypt` — stream cipher encrypt/decrypt (counter defaults to 1)
- `poly1305_mac` — 130-bit MAC using `Math::BigInt` for bignum arithmetic
- `aead_encrypt` — full AEAD construction (ChaCha20 + Poly1305 + length fields)
- `aead_decrypt` — authenticated decryption with constant-time tag comparison
- 18 test cases covering all RFC 8439 official test vectors:
  - §2.1.2: ChaCha20 block function
  - §2.4.2: ChaCha20 Sunscreen encryption
  - §2.5.2: Poly1305 "Cryptographic Forum Research Group"
  - §2.6.2: Poly1305 key generation
  - §2.8.2: Full AEAD ciphertext and tag
- Edge case tests: empty plaintext, single bytes, multi-block, AAD-only
- Security tests: bad tag rejection, tampered ciphertext rejection, tampered AAD rejection
- Extensive literate-programming inline documentation explaining ARX design,
  Poly1305 prime field, clamping, constant-time comparison, and AEAD construction
- Full POD documentation with security notes and usage examples
