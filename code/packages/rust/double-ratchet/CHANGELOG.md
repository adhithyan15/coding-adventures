# Changelog — coding_adventures_double_ratchet

## [0.1.0] — 2026-04-24

### Added

- `KeyPair` struct for X25519 ratchet key pairs. Implements `Zeroize` and `Drop`.
  Exposes `from_secret(secret: [u8;32])` constructor for use by caller-supplied keys.
- `MessageHeader` struct: `dh` (32), `pn` (u32), `n` (u32). Derives `Clone, Debug, PartialEq`.
- `Message` struct: decoded header + ciphertext (encrypted payload ‖ 16-byte tag).
  Derives `Debug, PartialEq`.
- `RatchetState` struct: full mutable session state. Implements `Drop` to zeroize
  root key, chain keys, and all cached skipped message keys.
- `RatchetError` enum with six variants.
- `MAX_SKIP = 1000` — maximum cached skipped message keys per direction.
- `HEADER_LEN = 40` — byte length of encoded `MessageHeader`.
- `generate_ratchet_keypair()` — fresh X25519 key pair with RFC 7748 clamping.
- `ratchet_init_alice()` — Alice-side initialization; performs initial DH ratchet
  step to establish the sending chain.
- `ratchet_init_bob()` — Bob-side initialization; waits for first message.
- `ratchet_encrypt()` — advances the symmetric KDF chain, builds the header,
  expands the message key to (enc_key, nonce), and AEAD-encrypts.
- `ratchet_decrypt()` — handles in-order and out-of-order messages; performs
  DH ratchet steps as needed.
- `encode_header()` / `decode_header()` — 40-byte serialization of `MessageHeader`.
- Internal helpers: `kdf_rk`, `kdf_ck`, `expand_message_key`, `skip_message_keys`,
  `dh_ratchet_step`, `try_skipped_message_key`.
- 15 unit tests: single message, multiple messages, bidirectional exchange,
  out-of-order delivery, AAD mismatch, tampered ciphertext, tampered header,
  header roundtrip, key derivation, empty/large plaintext, many exchanges,
  max-skip overflow, no-sending-chain error.
