# Changelog

## Unreleased

- Consume the versioned SE04 cross-language fixture for byte-identical
  HChaCha20, raw XChaCha20, AEAD, and authentication-failure conformance.

## 0.2.0

- Add HChaCha20 subkey derivation from the pinned SE04 construction
- Add raw XChaCha20 with 24-byte nonces and caller-selected counters
- Add XChaCha20-Poly1305 authenticated encryption and decryption
- Verify the draft HChaCha20 and Appendix A.3.1 vectors exactly
- Add negative, invalid-length, empty-message, and multi-block coverage

## 0.1.0

- Initial implementation of ChaCha20-Poly1305 (RFC 8439)
- ChaCha20 stream cipher with 256-bit key, 96-bit nonce, 32-bit counter
- Poly1305 one-time MAC using native BigInt arithmetic
- AEAD combined authenticated encryption and decryption
- All RFC 8439 test vectors verified (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for timing-attack resistance
- Input validation for key, nonce, and tag lengths
