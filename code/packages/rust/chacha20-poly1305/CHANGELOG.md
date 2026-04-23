# Changelog

## 0.2.0 — 2026-04-20

- Added HChaCha20 subkey derivation (draft-irtf-cfrg-xchacha §2.2).
- Added XChaCha20 stream cipher and XChaCha20-Poly1305 AEAD
  (draft-irtf-cfrg-xchacha §2.3), providing a 24-byte (192-bit) nonce
  that is safe to sample at random. This unblocks the Chief-of-Staff
  Vault spec (D18), where secrets are encrypted at rest using the
  extended-nonce variant.
- New public items: `hchacha20_subkey`, `xchacha20_encrypt`,
  `xchacha20_poly1305_aead_encrypt`, `xchacha20_poly1305_aead_decrypt`.
- Matches the draft §2.2.1 HChaCha20 test vector and the §A.3
  XChaCha20-Poly1305 gold-standard AEAD test vector. Added tamper,
  AAD-binding, long-message, and stream-cipher round-trip tests.

## 0.1.0

- Initial implementation of ChaCha20-Poly1305 (RFC 8439)
- ChaCha20 stream cipher with 256-bit key, 96-bit nonce, 32-bit counter
- Poly1305 one-time MAC using u128-based 130-bit arithmetic
- AEAD combined authenticated encryption and decryption
- All RFC 8439 test vectors verified (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for timing-attack resistance
