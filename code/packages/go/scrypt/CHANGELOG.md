# Changelog — go/scrypt

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of the scrypt password-based key derivation function
  per RFC 7914 (Colin Percival, 2009).
- `Scrypt(password, salt []byte, n, r, p, dkLen int) ([]byte, error)` — the
  primary API.  Returns a derived key of exactly `dkLen` bytes.
- `ScryptHex(password, salt []byte, n, r, p, dkLen int) (string, error)` —
  convenience wrapper returning a lowercase hex string.
- Internal `salsa20_8` — Salsa20/8 core function operating on 64-byte blocks.
- Internal `blockMix` — BlockMix per RFC 7914 §4.
- Internal `roMix` — ROMix per RFC 7914 §5, the sequential-memory-hard core.
- Internal `pbkdf2Sha256` — PBKDF2-HMAC-SHA256 without the empty-password
  restriction (required by RFC 7914 §12 test vector 1).
- Full input validation with named sentinel errors:
  `ErrInvalidN`, `ErrNTooLarge`, `ErrInvalidR`, `ErrInvalidP`,
  `ErrInvalidKeyLength`, `ErrKeyLengthTooLarge`, `ErrPRTooLarge`.
- RFC 7914 §12 test vectors 1 and 3 verified to pass exactly.
- Comprehensive test suite covering determinism, avalanche sensitivity, output
  length correctness, and all error branches.
- Literate programming style throughout — all algorithmic steps are explained
  inline with diagrams, worked examples, and design rationale.

### Implementation Notes

- Uses `hmacpkg.HMAC` directly (rather than `hmacpkg.HmacSHA256`) inside the
  internal PBKDF2 to support RFC 7914 vector 1's empty password.
- N is capped at 2^20 to prevent multi-gigabyte accidental allocations in test
  and development environments.
- ROMix processes `p` lanes sequentially; parallelisation across goroutines is
  left as a future optimisation.
