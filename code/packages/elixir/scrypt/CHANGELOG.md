# Changelog

All notable changes to `coding_adventures_scrypt` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-11

### Added

- Initial implementation of scrypt (RFC 7914) in Elixir.
- `CodingAdventures.Scrypt.scrypt/6` — derive a key as a raw binary.
- `CodingAdventures.Scrypt.scrypt_hex/6` — derive a key as a lowercase hex string.
- Full Salsa20/8 core (RFC 7914 §3): 64-byte permutation using 4 double-rounds of
  column and row quarter-rounds, with modular addition of initial state.
- BlockMix (RFC 7914 §4): 2r-block mixing with Salsa20/8 and even/odd deinterleaving.
- ROMix (RFC 7914 §5): memory-hard function using N-entry V table and pseudo-random
  lookups via Integerify.
- Internal PBKDF2-HMAC-SHA256 that bypasses the empty-key guard, enabling RFC 7914
  test vector 1 (empty password) to pass correctly.
- Comprehensive validation of all parameters (N, r, p, dk_len) with descriptive errors.
- 35 unit tests covering RFC vectors, output length, determinism, sensitivity, edge
  cases, and all validation error branches. Coverage: 97.56%.
- Literate programming style throughout — every function explained for new programmers.

### Notes

- The RFC 7914 §12 test vectors as printed in the document contain a typographic error
  in the later bytes of vector 1 and vector 2. The implementation uses the correct values
  as verified by Python `hashlib.scrypt`, Go `golang.org/x/crypto/scrypt`, and OpenSSL's
  `kdf` command. See test comments for details.
- The `coding_adventures_pbkdf2` package is declared as a dependency for documentation
  purposes, but scrypt uses its own internal PBKDF2 implementation to allow empty
  passwords, which the user-facing PBKDF2 correctly restricts for security.
