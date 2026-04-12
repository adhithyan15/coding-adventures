# Changelog — @coding-adventures/scrypt

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of `scrypt` (RFC 7914) in TypeScript.
- `scrypt(password, salt, n, r, p, dkLen)` — derives a key using the
  memory-hard scrypt algorithm. Accepts empty passwords (required by RFC 7914
  vector 1) via an internal PBKDF2-HMAC-SHA256 that bypasses the empty-key
  guard present in `@coding-adventures/pbkdf2`.
- `scryptHex(...)` — convenience wrapper returning hex-encoded output.
- Internal `salsa20_8` — 8-round Salsa20 block function (RFC 7914 §3) in
  little-endian byte order with `>>> 0` unsigned 32-bit arithmetic throughout.
- Internal `blockMix` — RFC 7914 §4 block mixing using Salsa20/8.
- Internal `roMix` — RFC 7914 §5 memory-hard random-oracle mixing (sequential
  fill phase + random read phase).
- Internal `pbkdf2Sha256Internal` — PBKDF2-HMAC-SHA256 (RFC 8018 §5.2) using
  `hmac(sha256, 64, ...)` directly, without empty-password restriction.
- Validation for all parameters: N power-of-2 check, N ≤ 2^20, r ≥ 1, p ≥ 1,
  dkLen ≥ 1, dkLen ≤ 2^20, p×r ≤ 2^30.
- Full test suite:
  - RFC 7914 §12 vector 1 (empty password + salt, N=16, r=1, p=1, dkLen=64).
  - RFC 7914 §12 vector 2 (password="password", salt="NaCl", N=1024, r=8, p=16, dkLen=64).
  - `scryptHex` correctness and RFC vector match.
  - Output length correctness for multiple dkLen values.
  - Prefix property (short output matches prefix of longer output).
  - Determinism.
  - Parameter sensitivity (password, salt, N, r, p each influence output).
  - All error cases.
- Literate programming comments throughout `src/index.ts` explaining the
  Salsa20/8 quarter-round, BlockMix interleaving, ROMix phases, PBKDF2
  block construction, and JavaScript uint32 arithmetic rules.
