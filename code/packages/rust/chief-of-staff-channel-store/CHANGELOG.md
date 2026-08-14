# Changelog

## Unreleased

- Expose the production D18S state and D18A cursor codecs, normative content
  types and bounds, and stable D18P error codes through a public compatibility
  module.
- Consume the channel-crypto package's structurally immutable message envelope
  through its read-only accessors.

## 0.1.0

- Add CAS-protected durable next-sequence and pending-header state.
- Add reserve-before-encrypt, idempotent ciphertext commit, and safe abandoned
  sequence gaps.
- Add ordered encrypted-message reads and monotonic per-receiver acknowledgements.
- Add idempotent sealed key-grant persistence.
