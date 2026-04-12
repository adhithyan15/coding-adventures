# Changelog — CodingAdventures::Scrypt (Perl)

## 0.1.0 — 2026-04-11

### Added
- Initial implementation of scrypt (RFC 7914) for Perl 5.26+.
- `scrypt($password, $salt, $n, $r, $p, $dk_len)` — returns raw binary string.
- `scrypt_hex(...)` — returns lowercase hex string.
- Full RFC 7914 § 11 test vectors (vector 1: empty password/salt; vector 2: "password"/"NaCl"). Expected values verified against Python 3 hashlib.scrypt (OpenSSL backend).
- Inline `_pbkdf2_sha256_raw` to handle empty passwords (RFC 7914 vector 1 uses `""`, which `CodingAdventures::PBKDF2` correctly rejects).
- Salsa20/8 core with explicit `& 0xFFFFFFFF` masking throughout to prevent 64-bit Perl integer overflow.
- `_block_mix` implementing the RFC 7914 § 3 BlockMix algorithm with even/odd shuffle.
- `_ro_mix` implementing RFC 7914 § 4 ROMix with N-slot scratchpad and data-dependent reads.
- Parameter validation: N power-of-2, N >= 2, N <= 2^20, r >= 1, p >= 1, dk_len in [1, 2^20], p*r <= 2^30.
- Uses `use bytes;` pragma so `length()` and `substr()` operate on raw octets.
- Uses `pack("V16", ...)` / `unpack("V16", ...)` for little-endian 32-bit word I/O.
- Literate programming (Knuth-style) comments throughout, including quarter-round diagram and memory-requirement formulae.
