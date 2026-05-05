# Changelog

All notable changes to this package are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-05-04

### Added

- Initial implementation of VLT14
  (`code/specs/VLT14-vault-attachments.md`).
- `BlobId` — 128-bit opaque identifier with hex `Debug`.
- `BlobReference` — carries `(blob_id, dek, total_plaintext_len, chunk_count)`.
  DEK held in `Zeroizing<[u8; 32]>`. Hand-rolled redacted
  `Debug` so `dbg!` cannot leak the key.
- `EncryptedChunk` — `(blob_id, index, is_final, ciphertext, tag)`.
  Hand-rolled redacted `Debug` (lengths only) so logs cannot
  fingerprint chunk content.
- `AttachmentEncryptor`:
  - `new_random()` draws a 128-bit blob id + 256-bit DEK from
    the CSPRNG.
  - `from_dek(blob_id, dek)` for callers who already have a
    DEK (e.g. wrapped by VLT04 above).
  - `encrypt_chunk(plaintext, is_final)` consumes ≤ 64 KiB of
    plaintext and returns an `EncryptedChunk`. Refuses
    intermediate empty chunks (final chunk may be empty);
    refuses calls after `is_final`; counts towards
    `MAX_CHUNK_COUNT` and `MAX_PLAINTEXT_LEN` via `checked_add`.
  - `finalize_reference()` returns the `BlobReference` once
    `is_final` has been seen.
- `AttachmentDecryptor`:
  - `new(blob_id, dek)` and `from_reference(&blob_ref)`.
  - `decrypt_chunk(&chunk)` enforces
    `chunk.blob_id == self.blob_id`, strict ascending index,
    and AEAD verification (XChaCha20-Poly1305) on the
    counter-derived nonce + the AAD.
  - `finish()` returns `Truncated` if no `is_final` chunk was
    seen — catches a server that drops the tail.
  - Refuses chunks after `is_final` → `AfterFinal`.
- AEAD nonce layout: `blob_id(16) || chunk_index_be(4) || 0000 0000`.
- AEAD AAD layout: `"VAT1" || blob_id || chunk_index_be || is_final_byte || total_plaintext_len_be`.
  Binding `chunk_index` and `is_final` to AAD means a server
  cannot reorder chunks or swap an intermediate into the
  final position.
- `AttachmentError` — `InvalidParameter` / `Misuse` / `Aead`
  / `Truncated` / `AfterFinal` / `Csprng` / `TooLarge`.
- `#![forbid(unsafe_code)]` + `#![deny(missing_docs)]`.
- 26 unit tests covering: single-chunk roundtrip,
  multi-chunk roundtrip, empty final chunk, finalize-
  reference metadata, `new_random` yields unique ids/keys,
  cannot encrypt after final, oversize chunk rejection,
  empty intermediate chunk rejection, finalize-before-final
  errors, tampered ciphertext / tag → `Aead`, reordered
  chunks → `Misuse`, cross-blob chunk → `Misuse`, intermediate-
  to-final and final-to-intermediate flag flips → `Aead`,
  truncation caught by `finish()`, cannot decrypt after final,
  `new_random` round-trip via `finalize_reference`,
  chunk-count overflow protection, `BlobReference` Debug
  redacts DEK, `EncryptedChunk` Debug redacts ciphertext,
  `BlobId` Debug is hex, AAD swap on chunk index breaks
  verify, forged chunk with swapped index fails AEAD,
  encryptor + decryptor are `Send`.

### Security hardening (pre-merge review)

Three findings flagged before push, all fixed inline:

- **LOW** — Decryptor only relied on `u32::MAX` overflow for
  chunk-count protection while the encryptor enforced
  `MAX_CHUNK_COUNT` (~2^24). A peer encryptor producing more
  than the documented cap with valid AEAD tags would not be
  rejected at policy until 4.29 B chunks. Mirrored the
  encryptor's bound on the decryption side; also added a
  `MAX_PLAINTEXT_LEN` check on `total_plaintext_seen`. New
  test `decryptor_rejects_chunks_past_max_chunk_count`.
- **LOW** — `from_dek` was documented as merely "useful when
  the blob id is content-derived". A content-derived blob id
  + deterministic DEK would silently re-use the same nonce
  across uploads of the same file — catastrophic for
  XChaCha20-Poly1305 (key-stream reuse → plaintext recovery
  via XOR; Poly1305 tag-derivation key recovery → arbitrary
  forgery). Strengthened the doc comment to a full
  "**Caller responsibility — nonce uniqueness**" block that
  lists safe + unsafe usage patterns explicitly and points
  back to `new_random` as the default.
- **INFO** — `aad_total = 0` was previously commented as
  making `total_plaintext_len` "opaque-but-authenticated;
  not a length oracle". Misleading: with `aad_total = 0` in
  AAD, the `total_plaintext_len` field on `BlobReference` is
  *not* authenticated by chunk AEAD. Replaced the comment
  with an explicit note that callers MUST verify
  `dec.finish()? == blob_ref.total_plaintext_len`
  themselves if they want to catch tampering of that hint.

### Bounds

`CHUNK_SIZE = 64 KiB`, `MAX_CHUNK_COUNT = 1 << 24`,
`MAX_PLAINTEXT_LEN = CHUNK_SIZE * MAX_CHUNK_COUNT`,
`DEK_LEN = 32`, `TAG_LEN = 16`, `BLOB_ID_LEN = 16`.

### Out of scope (future PRs)

- **Wire transport** — VLT11 carries `EncryptedChunk`s over
  HTTP/gRPC.
- **Multi-recipient DEK wrap** — VLT04 wraps the DEK before
  storage. This crate hands it to the host as
  `Zeroizing<[u8; 32]>`; the host wraps before persistence.
- **Async streaming traits** — the API is sync; an async
  wrapper landing on top is straightforward.
- **Resumable upload** — the chunks are independently
  AEAD-tagged so resumable upload is naturally supported by
  the host's storage layer; this crate doesn't provide
  anything beyond the per-chunk verification.
- **Content-deduplication** — same plaintext gets distinct
  blob ids by design (per-blob DEK).
- **Compression before encryption** — out of scope; if
  needed, the host compresses before calling `encrypt_chunk`.
- **Forward-secure framing** — at-rest sealing protects each
  chunk until the master key falls; forward-secure stream
  primitives (each chunk's key derived from the previous) are
  future work.
