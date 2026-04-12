# Changelog — go/aes

All notable changes to this package will be documented in this file.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of AES (Advanced Encryption Standard) conforming to FIPS 197.
- `SBOX` and `INV_SBOX` — 256-byte S-box tables built at init time from GF(2^8)
  multiplicative inverses (using polynomial 0x11B) followed by the AES affine
  transformation. `INV_SBOX[SBOX[b]] == b` for all 256 byte values.
- `ExpandKey` — key schedule for AES-128, AES-192, and AES-256, producing 11, 13,
  or 15 round keys respectively. Each round key is a [4][4]byte matrix (row-major).
  Implements SubWord, RotWord, and the AES-256 extra SubWord step per FIPS 197 §5.2.
- `EncryptBlock` / `DecryptBlock` — 16-byte block cipher for all three key sizes.
  Encryption: AddRoundKey → (SubBytes, ShiftRows, MixColumns, AddRoundKey) × (Nr-1)
              → SubBytes, ShiftRows, AddRoundKey.
  Decryption: AddRoundKey → (InvShiftRows, InvSubBytes, AddRoundKey, InvMixColumns) × (Nr-1)
              → InvShiftRows, InvSubBytes, AddRoundKey.
- Depends on `go/gf256` via `gf256.NewField(0x11B)` for GF(2^8) arithmetic
  (S-box inverse computation, xtime/MixColumns, Rcon generation).
- Comprehensive test suite: FIPS 197 Appendix B and C.1/C.2/C.3 known-answer vectors
  for all three key sizes, S-box properties (bijection, no fixed points, known values),
  key schedule shape and correctness, round-trip tests, error handling, and an
  avalanche-effect check.
