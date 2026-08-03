# Changelog

## 0.1.0

- Add zeroizing channel master, X25519 receiver, and Ed25519 originator key types.
- Add signed, receiver-bound X25519/HKDF/XChaCha20-Poly1305 channel-key grants.
- Add receiver epoch-key installation with idempotent retry and conflict checks.
- Add canonical message headers, deterministic channel/sequence nonces, and
  authenticated message encryption.
- Add a fail-closed durable sequence recovery and reservation cursor.
- Declare the `os_random` capability used for CMKs, ephemeral keys, and nonces.
