# Changelog

## [Unreleased]

### Added

- Consume the versioned SE04 cross-language fixture for byte-identical
  HChaCha20, raw XChaCha20, AEAD, and authentication-failure conformance.
- Add `Jason` as a test-only dependency; runtime dependencies remain zero.

## [0.2.0] - 2026-08-13

### Added

- `hchacha20_subkey/2` using the shared ChaCha20 20-round permutation without feed-forward.
- `xchacha20_encrypt/4` for raw SE04 XChaCha20 at a caller-selected 32-bit counter.
- `xchacha20_poly1305_encrypt/4` and `xchacha20_poly1305_decrypt/5`, delegating to the existing RFC 8439 AEAD core after HChaCha20 derivation.
- SE04 draft gold vectors, negative authentication cases, boundary tests, exact input-length checks, and counter validation.

### Security

- Documented that the expired XChaCha Internet-Draft is a pinned construction reference and that complete 24-byte nonces must remain unique per key.
- Documented the BEAM runtime's lack of guaranteed in-place zeroization for immutable derived-subkey binaries.

## [0.1.0] - 2026-04-12

### Added

- `chacha20_block/3` — ChaCha20 block function (RFC 8439 §2.1). Generates a single 64-byte keystream block from a 256-bit key, 32-bit counter, and 96-bit nonce. Verified against RFC 8439 §2.1.2 test vector.
- `chacha20_encrypt/4` — Stream cipher encryption/decryption via XOR with ChaCha20 keystream. Verified against RFC 8439 §2.4.2 test vector and Erlang OTP `:crypto` reference implementation.
- `poly1305_mac/2` — Poly1305 one-time MAC over GF(2^130 - 5). Implements r-clamping and little-endian serialisation per RFC 8439 §2.5. Verified against RFC 8439 §2.5.2 test vector.
- `aead_encrypt/4` — AEAD encrypt returning `{ciphertext, tag}`. Derives Poly1305 key from ChaCha20 block 0, encrypts with blocks 1+, builds MAC input per RFC 8439 §2.8.
- `aead_decrypt/5` — AEAD decrypt with constant-time tag verification. Returns `{:ok, plaintext}` or `{:error, :authentication_failed}`. Verified against RFC 8439 §2.8.2 test vector.
- 34 unit tests covering RFC test vectors, round-trip correctness, boundary conditions (0, 1, 63, 64, 65, 200, 10240 bytes), and authentication failure modes.
- 98.48% test coverage (exceeds the 95% library target).
- Literate-programming style inline documentation explaining ARX operations, Poly1305 polynomial evaluation, and AEAD construction rationale.
