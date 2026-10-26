# Changelog — coding_adventures_hmac

## [Unreleased]

### Fixed

- **Key-derived intermediates are now wiped.** `hmac()` built `key_prime`,
  the two padded keys, and the two nested inputs in plain `Vec<u8>`s that were
  simply dropped. `K' XOR ipad` is not a one-way function of the key — XOR
  0x36 back out and you have `K'` — so each call returned several recoverable
  images of the secret to the allocator. Every one of those buffers, plus
  `normalize_key`'s intermediate hash and the inner digest, is now wipe-on-drop.

- **The padded keys no longer leave an abandoned copy behind.** They were built
  with `collect()` at exact block-size capacity and then grown with
  `extend_from_slice(message)`, which reallocates: the first buffer, holding
  `K' XOR ipad`, was freed un-wiped and *would not have been fixed* by wrapping
  the final buffer in `Zeroizing`, since by then it is the wrong copy. Each
  nested input is now allocated at its exact final size and filled in place.

- **The named variants wipe their tag buffer.** `hmac_*` computed into a heap
  `Vec` and moved it into a fixed-size array with `try_into`, freeing the `Vec`
  with a live authentication tag still in it. They now copy and zeroize. The
  returned array is the caller's to wipe.

  This matters most for the caller that motivated the audit: a password
  manager displaying TOTP codes re-hashes one long-lived stored seed on every
  request, so a per-call leak is a repeated leak of the same shared secret. The
  fix is in this crate rather than that one, so every HMAC caller gets it.

  No output changes — the RFC 2202/4231 vectors, and RFC 6238's Appendix B
  vectors downstream, are unchanged.

### Added

- `coding_adventures_zeroize` dependency, for the above.

## [0.1.0] — 2026-04-06

### Added

- `hmac<F>(hash_fn, block_size, key, message) -> Vec<u8>` — generic HMAC over any hash function
- `hmac_md5(key, message) -> [u8; 16]` — HMAC-MD5 (RFC 2202)
- `hmac_sha1(key, message) -> [u8; 20]` — HMAC-SHA1 (RFC 2202)
- `hmac_sha256(key, message) -> [u8; 32]` — HMAC-SHA256 (RFC 4231)
- `hmac_sha512(key, message) -> [u8; 64]` — HMAC-SHA512 (RFC 4231)
- `hmac_md5_hex`, `hmac_sha1_hex`, `hmac_sha256_hex`, `hmac_sha512_hex` — hex-string variants
- Full test suite: RFC 4231 TC1–TC3, TC6, TC7 for SHA-256/SHA-512; RFC 2202 TC1, TC2, TC6 for MD5/SHA-1
- Key normalisation: keys longer than block_size are pre-hashed; all keys zero-padded to block_size
- Literate documentation explaining ipad/opad choice, length extension attacks, and Merkle-Damgård weakness
