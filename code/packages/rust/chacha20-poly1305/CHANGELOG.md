# Changelog

## Unreleased

- Add the allocation-free `poly1305_mac_fixed` boundary with caller-owned tag
  output, a branchless five-limb block core and final reduction, wipe-on-drop
  derived state, and RFC 8439, independent multi-block, plus block-edge
  equivalence coverage.
- Add the allocation-free `chacha20_xor_fixed` boundary with exact
  caller-owned output, preflighted 32-bit counter exhaustion, stable
  payload-free errors, and fixed-round secret processing.
- Retain ChaCha20 original/working word states, keystream blocks, AEAD
  one-time Poly1305 keys, and XChaCha20 subkeys in wipe-on-drop owners.
- Add the repository-owned `coding_adventures_zeroize` runtime dependency;
  no external production dependency is introduced.
- Consume the versioned SE04 cross-language fixture for byte-identical
  HChaCha20, raw XChaCha20, AEAD, and authentication-failure conformance.
- Add `serde_json` as a test-only dependency.

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
