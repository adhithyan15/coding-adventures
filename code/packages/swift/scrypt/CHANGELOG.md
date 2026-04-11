# Changelog — swift/scrypt

All notable changes to this package follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.

---

## [0.1.0] — 2026-04-11

### Added

- Initial implementation of the scrypt password-based key derivation function (RFC 7914).
- `scrypt(password:salt:n:r:p:dkLen:)` — public API returning `[UInt8]`.
- `scryptHex(password:salt:n:r:p:dkLen:)` — convenience wrapper returning a lowercase hex string.
- `ScryptError` enum with cases: `invalidN`, `nTooLarge`, `invalidR`, `invalidP`, `invalidKeyLength`, `prTooLarge`, `hmacError`.
- Internal `salsa20_8` — Salsa20/8 mixing function operating on 64-byte blocks with wrapping UInt32 arithmetic.
- Internal `blockMix` — RFC 7914 §4 BlockMix over 2r blocks.
- Internal `roMix` — RFC 7914 §5 ROMix memory-hard function (O(N*r) space).
- Internal `pbkdf2HmacSHA256Internal` — PBKDF2-HMAC-SHA256 without empty-key guard, supporting RFC 7914 vector 1 (empty password).
- Full RFC 7914 §11 test vectors 1 and 2 verified.
- Parameter validation tests (N not power of 2, N too large, r=0, p=0, dkLen=0, dkLen too large, p*r overflow).
- Determinism, sensitivity, and output-length correctness tests.
- Literate programming style throughout — Knuth-style prose explaining every design decision.

### Implementation Notes

- The PBKDF2 package (code/packages/swift/pbkdf2) rejects empty passwords as a security policy. scrypt implements PBKDF2-HMAC-SHA256 inline using the lower-level `hmac()` function from the HMAC package (which has no empty-key precondition) to support RFC 7914 vector 1.
- Package.swift uses `swift-tools-version: 6.0` consistent with other packages in the stack.
- Dependencies: `../hmac` (for `hmac()`) and `../sha256` (for `sha256` hash function passed to `hmac()`).
- RFC 7914 vector 3 (N=16384) is omitted from CI due to ~5s runtime but documented as a comment in the test file.
- Salsa20/8 row round uses proper Salsa20 spec indices for rows 2 and 3: (10,11,8,9) and (15,12,13,14) respectively. The user's prompt contained incorrect row round indices; the correct indices were confirmed by cross-referencing against Python's `hashlib.scrypt` which produces output matching the actual RFC 7914 §11 test vectors.
- The test vector hex strings in the user's prompt were also incorrect. Correct values were taken from RFC 7914 §11 and verified with Python's `hashlib.scrypt`.
