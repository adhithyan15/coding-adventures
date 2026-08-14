# Changelog

## Unreleased

- Consume the channel-crypto package's structurally immutable message envelope
  through its read-only accessors.

## 0.1.0

- Add CAS-protected durable next-sequence and pending-header state.
- Add reserve-before-encrypt, idempotent ciphertext commit, and safe abandoned
  sequence gaps.
- Add ordered encrypted-message reads and monotonic per-receiver acknowledgements.
- Add idempotent sealed key-grant persistence.
