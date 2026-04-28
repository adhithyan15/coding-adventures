# Changelog — coding_adventures_sealed_sender

## [0.1.0] — 2026-04-24

### Added

- `SenderCertificate` struct — 124-byte wire format: uuid (16) + device_id (4) +
  ik_public (32) + expires_at (8) + server_sig (64). Derives `Clone, Debug, PartialEq`.
- `SealedMessage` struct — `recipient_token: [u8;32]` + `envelope: Vec<u8>`.
- `SealedError` enum with six variants; implements `Display`, `Error`, and
  `From<RatchetError>`.
- `CERT_LEN = 124`, `CERT_SIGNED_LEN = 60` constants.
- `encode_cert()` / `decode_cert()` — 124-byte serialization.
- `issue_sender_certificate()` — server-side certificate issuance with Ed25519
  signature over the first 60 bytes.
- `verify_cert_signature()` — Ed25519 verification helper.
- `derive_recipient_token()` — HKDF-SHA256 derivation of the server routing token.
- `sealed_send()` — ephemeral X25519 key generation, ECDH, HKDF key expansion,
  inner payload encoding, and AEAD encryption of the full envelope.
- `sealed_receive()` — envelope decryption, payload decoding, certificate
  verification, expiry check, and Double Ratchet decryption.
- Internal `encode_payload()` / `decode_payload()` — inner wire format with
  overflow-checked offset arithmetic to prevent integer overflow on adversarial
  payloads.
- Re-exports from `coding_adventures_x3dh` and `coding_adventures_double_ratchet`
  for convenient one-import usage.
- 10 unit tests: full stack integration, multiple messages, expired certificate,
  tampered envelope, invalid server signature, wrong recipient key, deterministic
  token, distinct tokens, cert encode/decode roundtrip, AAD mismatch.

### Security Notes

- Certificate expiry check uses `now_ms >= cert.expires_at` (not `<`) to
  correctly reject certificates at or past their expiry timestamp.
- All ephemeral secrets (eph_secret, dh_out, enc_key) held in `Zeroizing<>`
  RAII wrappers — wiped from memory even on panic.
- Payload decode uses `checked_add` throughout to prevent u32 overflow on
  crafted cert_len/ct_len fields.
