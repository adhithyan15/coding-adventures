# Changelog — coding_adventures_scrypt

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Versioning follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of scrypt key derivation function (RFC 7914).
- `CodingAdventures::Scrypt.scrypt(password, salt, n, r, p, dk_len)` — returns
  a binary String of dk_len bytes derived by the scrypt algorithm.
- `CodingAdventures::Scrypt.scrypt_hex(password, salt, n, r, p, dk_len)` — same
  as `scrypt` but returns a lowercase hex string.
- Full literate-programming inline documentation for every function: rotl32,
  quarter_round, salsa20_8, xor64, block_mix, integerify, ro_mix, and the
  inline PBKDF2-HMAC-SHA256 implementation.
- Inline PBKDF2-HMAC-SHA256 that bypasses the non-empty-key guard so that
  RFC 7914 vector 1 (empty password) is supported correctly.
- Parameter validation with descriptive ArgumentError messages for invalid N
  (not a power of 2, out of range), r < 1, p < 1, dk_len out of range, and
  p * r exceeding 2^30.
- Minitest suite covering:
  - RFC 7914 vectors 1 and 2 (ground-truth correctness).
  - Hex variant consistency.
  - Output length and binary encoding.
  - Determinism across two identical calls.
  - Sensitivity to password, salt, r, and p changes.
  - All invalid-parameter error cases.
  - Edge cases: minimum N (2), minimum dk_len (1), binary input, UTF-8 input.
- `Gemfile`, `Rakefile`, `BUILD`, `required_capabilities.json`, `README.md`,
  `CHANGELOG.md` per monorepo standards.

### Implementation Notes

- All Salsa20/8 arithmetic uses `& 0xFFFFFFFF` after every add/rotate/XOR to
  stay within uint32 range (Ruby integers are arbitrary-precision).
- Binary encoding is enforced with `.b` at all string boundaries.
- `CodingAdventures::Hmac.hmac` is called directly (not `hmac_sha256`) so that
  the empty-password case in RFC 7914 vector 1 is handled without modification
  to the HMAC gem.
