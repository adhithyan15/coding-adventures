# Changelog — coding_adventures_aes (Rust)

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of AES block cipher (FIPS 197) in Rust.
- Depends on `gf256` crate for `Field::new(0x11B)` — the AES GF(2^8) field.
- `expand_key` — FIPS 197 §5.2 key schedule supporting AES-128, AES-192, AES-256.
  Returns `(Nr+1)` round keys each as a `[[u8; 4]; 4]` column-major state matrix.
- `encrypt_block` — AES encryption: AddRoundKey → (Nr-1) × (SubBytes + ShiftRows
  + MixColumns + AddRoundKey) → SubBytes + ShiftRows + AddRoundKey.
- `decrypt_block` — AES decryption with inverted operations in reverse order:
  InvShiftRows + InvSubBytes + AddRoundKey + InvMixColumns.
- `sbox()` / `inv_sbox()` — lazily-initialized via `OnceLock`; thread-safe.
  SBOX built from GF(2^8) multiplicative inverse + AES affine transformation.
- All FIPS 197 Appendix B, C.1, C.2, C.3 known-answer vectors pass.
- S-box is a bijection with no fixed points (verified by tests).
- Full round-trip coverage for all three key sizes across diverse plaintexts.
