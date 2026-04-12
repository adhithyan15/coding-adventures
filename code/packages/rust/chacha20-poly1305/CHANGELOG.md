# Changelog

## 0.1.0

- Initial implementation of ChaCha20-Poly1305 (RFC 8439)
- ChaCha20 stream cipher with 256-bit key, 96-bit nonce, 32-bit counter
- Poly1305 one-time MAC using u128-based 130-bit arithmetic
- AEAD combined authenticated encryption and decryption
- All RFC 8439 test vectors verified (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for timing-attack resistance
