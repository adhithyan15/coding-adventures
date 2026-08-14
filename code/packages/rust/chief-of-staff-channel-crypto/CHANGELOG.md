# Changelog

## Unreleased

- Add the portable D18F profile adapter with high-level UUID-v7/MIME
  validation, canonical lossless JSON, stable error codes, epoch-key
  resolution, deterministic UUID-v7 source injection, and shared fixture
  generation.
- Make authenticated message fields, headers, ciphertexts, tags, and signatures
  structurally immutable outside the crate, with read-only constructors and
  accessors for channel-store and endpoint callers.
- Add versioned, bounded binary codecs for sealed key grants and encrypted
  channel-log messages.
- Add stable storage keys for messages, key grants, sequence state, and
  receiver acknowledgements.
- Add authenticated-header preparation, encoding, and post-reservation
  encryption for durable nonce-safe appends.
- Add a stable durable channel-definition storage key for authorized endpoint
  membership.

## 0.1.0

- Add zeroizing channel master, X25519 receiver, and Ed25519 originator key types.
- Add signed, receiver-bound X25519/HKDF/XChaCha20-Poly1305 channel-key grants.
- Add receiver epoch-key installation with idempotent retry and conflict checks.
- Add canonical message headers, deterministic channel/sequence nonces, and
  authenticated message encryption.
- Add a fail-closed durable sequence recovery and reservation cursor.
- Declare the `os_random` capability used for CMKs, ephemeral keys, and nonces.
