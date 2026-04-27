# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added
- `aes_encrypt_block` / `aes_decrypt_block` — AES block cipher for 128/192/256-bit keys (10/12/14 rounds)
- `expand_key` — AES key schedule: RotWord, SubWord, Rcon XOR expansion to (Nr+1) round keys
- `SBOX` / `INV_SBOX` — 256-byte S-box and inverse, generated from GF(2^8) multiplicative inverse + affine transform
- AES-specific GF(2^8) field using `GF256Field(0x11B)` from the `coding-adventures-gf256` package
- `_mix_col` / `_inv_mix_col` — MixColumns and InvMixColumns using the AES matrix in GF(2^8)
- Comprehensive test suite: FIPS 197 Appendix B, C.1, C.2, C.3 test vectors; S-box bijectivity; key schedule; round-trip; avalanche diffusion
