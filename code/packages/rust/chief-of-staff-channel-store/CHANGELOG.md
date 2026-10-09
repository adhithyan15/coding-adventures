# Changelog

## Unreleased

- Add `abandon_pending_at(sequence)` (D18S P2.6d-1): abandons the pending
  append only if it is at that sequence, so a broker gives back the
  reservation it made, and no other.
- Add `reserve_append_with_hash` and `commit_encrypted` (D18S P2.6d): the
  storage half of an append, needing no secret key. `commit_encrypted`
  takes the originator's public key and refuses a message whose signature
  does not verify, before anything is written.
  - `reserve_append` delegates to `reserve_append_with_hash`.
  - `commit_reserved` now encrypts first, then delegates to
    `commit_encrypted`. Crash recovery there is a byte comparison, which is
    sound because encryption is deterministic.
  - Error order changes in one case only: when both the header and the
    plaintext are wrong, the plaintext's mismatch is reported.
  - A test shows the split writes byte-identical records to `append`.
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
