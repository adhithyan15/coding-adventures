# Changelog

## 0.1.0

- Initial implementation of ChaCha20-Poly1305 (RFC 8439)
- ChaCha20 stream cipher with 256-bit key, 96-bit nonce, 32-bit counter
- Poly1305 one-time MAC using radix-2^26 limb arithmetic
- AEAD combined authenticated encryption and decryption
- All RFC 8439 test vectors verified (Sections 2.4.2, 2.5.2, 2.8.2)
- Constant-time tag comparison for timing-attack resistance
- Swift 6 compatible (strict concurrency)
