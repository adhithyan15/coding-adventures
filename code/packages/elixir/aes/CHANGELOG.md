# Changelog — coding_adventures_aes (Elixir)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `CodingAdventures.Aes` (SE02).
- `aes_encrypt_block/2` — AES-128/192/256 block encryption (FIPS 197).
- `aes_decrypt_block/2` — AES block decryption with inverse operations.
- `expand_key/1` — key schedule producing 11/13/15 round keys for AES-128/192/256.
- `sbox/0` and `inv_sbox/0` — S-box and inverse S-box accessors.
- GF(2^8) arithmetic (polynomial 0x11B) inline with xtime/gf_mul helpers.
- Hardcoded FIPS 197 S-box and inverse S-box tuples for O(1) lookups.
- FIPS 197 Appendix B and C test vectors (AES-128, AES-192, AES-256).
- SP 800-38A AES-256 test vector.
- Literate-programming comments explaining the SPN structure, GF arithmetic,
  state layout, and key schedule.
