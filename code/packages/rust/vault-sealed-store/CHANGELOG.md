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
  re-encrypting bodies; restartable, crash-safe, and back-compat with
  the retired KEK's password (the retired entry keeps its own salt
  and verifier so both passwords can unseal during and after rotation).
- Reserved namespace `__vault__` for vault-internal records, including
  a namespace registry side record `(__vault__, namespaces)` that
  drives rotation's per-namespace walk.
- In-memory KEK held in a `Zeroizing<[u8; 32]>`; `seal()` and `drop`
  wipe the key. All ephemeral DEKs and KEKs are wrapped in `Zeroizing`
  at allocation so early-return error paths cannot leak key material.
- Hard upper bounds on Argon2id parameters read from the manifest
  (time_cost ≤ 10, memory ≤ 5 GiB, parallelism ≤ 64) — a tampered
  at-rest manifest cannot stall unseal indefinitely.
- `Validation` error messages are sourced only from literals in this
  crate, never from persisted bytes, so they cannot carry attacker
  payloads.
- 22 unit tests covering roundtrips, tamper detection (body corruption,
  body swap, address rewrite), wrong-password rejection, reserved
  namespace rejection, CAS conflicts, empty / large plaintexts, list
  without decryption, re-unseal across store instances, Argon2 param
  bounds (both caller-supplied and manifest-sourced), duplicate-KEK-id
  rejection, namespace-registry tamper defence, and full `rotate_kek`
  across multiple namespaces with retired-KEK recovery semantics.
