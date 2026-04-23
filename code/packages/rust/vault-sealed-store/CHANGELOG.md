# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-04-22

### Added

- Initial implementation of VLT01 (`code/specs/VLT01-vault-sealed-store.md`).
- `SealedStore` facade wrapping any `storage_core::StorageBackend`.
- Seal / unseal ceremony using Argon2id for key derivation and a
  known-plaintext verifier stored in the manifest.
- Per-record envelope encryption with XChaCha20-Poly1305:
  - Fresh CSPRNG-drawn DEK per record, wrapped under the master KEK.
  - AEAD AAD binds the ciphertext to its `(namespace, key)` slot.
  - Wrapped-DEK AEAD AAD also binds to the KEK id, so swaps across
    KEKs are detected.
- `put` / `get` / `delete` / `list` data-plane operations.
- `rotate_kek` support: re-wraps DEKs under a new master KEK without
  re-encrypting bodies; restartable via per-record `kek_id` metadata.
- Reserved namespace `__vault__` for vault-internal records.
- In-memory KEK held in a `Zeroizing<[u8; 32]>`; `seal()` and `drop`
  wipe the key.
- 18 unit tests covering roundtrips, tamper detection (body corruption,
  body swap, address rewrite), wrong-password rejection, reserved
  namespace rejection, CAS conflicts, empty / large plaintexts, list
  without decryption, and re-unseal across store instances.
