# Changelog — coding-adventures-scrypt

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of the scrypt key derivation function (RFC 7914).
- `scrypt(password, salt, N, r, p, dk_len)` — returns a raw binary string.
- `scrypt_hex(password, salt, N, r, p, dk_len)` — returns a lowercase hex string.
- Internal pure-Lua Salsa20/8 core with correct 32-bit wrapping arithmetic.
- BlockMix and ROMix routines matching RFC 7914 §§3–4.
- Internal PBKDF2-HMAC-SHA256 that allows empty passwords (required by RFC 7914
  test vector 1), bypassing the public `hmac_sha256` empty-key guard by calling
  the generic `hmac.hmac()` engine directly.
- Full input validation: N power-of-2 ≥ 2 and ≤ 2^20, r ≥ 1, p ≥ 1,
  dk_len in [1, 2^20], p×r ≤ 2^30.
- Both RFC 7914 §11 test vectors verified (vector 1: empty password/salt;
  vector 2: "password"/"NaCl", N=1024, r=8, p=16).
- 24-test suite with full coverage of validation, output properties, edge
  cases, and hex formatting.
- Literate-programming inline documentation explaining scrypt design rationale,
  Salsa20/8 internals, memory-hardness, and Lua 5.4 bit-arithmetic.
