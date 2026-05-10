# VLT14 — Vault Attachments

## Overview

Streamable encrypted blob attachments. Multi-MiB photos /
exports / document scans don't fit in a sealed-store record (a
1 MiB record gets sync'd in full on every change, and holding
the whole plaintext in RAM is wrong for a phone). VLT14 splits
each blob into fixed-size 64 KiB chunks, each AEAD-encrypted
under a per-blob DEK with counter-derived nonces.

The shape borrows from age v1's stream format and HashiCorp
Vault's transit framing.

Implementation lives at `code/packages/rust/vault-attachments/`.

## Why this layer exists

Without VLT14, the only way to attach a 50 MiB document to a
record would be to inline it. That's pathological for sync
(50 MiB re-uploaded on every change), pathological for
on-device memory (RAM footprint linear in attachment count),
and pathological for forensics (no way to share a single big
file across multiple records without duplicating).

VLT14 makes attachments first-class:

- **Per-blob DEK**: each blob has its own 256-bit key.
  Compromise of one DEK doesn't compromise others.
- **Chunked + counter-nonced**: 64 KiB chunks, each its own
  AEAD nonce derived from `(blob_id, chunk_index)`. RAM stays
  bounded on read.
- **Reference, not inline**: parent records store a
  `BlobReference` (blob_id + DEK + total_plaintext_len +
  chunk_count); the chunks live in a sibling namespace
  (`__vault_blobs__/<blob_id>/<chunk_index>`).
- **Stream API**: `encrypt_chunk` / `decrypt_chunk` consume
  one chunk at a time. The host pipes plaintext from disk
  through the encryptor and back without ever buffering the
  whole blob.

## Public API

```rust
pub struct BlobId([u8; 16]);

pub struct BlobReference {
    pub blob_id: BlobId,
    pub dek: Zeroizing<[u8; 32]>,
    pub total_plaintext_len: u64,
    pub chunk_count: u32,
}

pub struct EncryptedChunk {
    pub blob_id: BlobId,
    pub index: u32,
    pub is_final: bool,
    pub ciphertext: Vec<u8>,
    pub tag: [u8; 16],
}

pub struct AttachmentEncryptor;
impl AttachmentEncryptor {
    pub fn new_random() -> Result<Self, AttachmentError>;
    pub fn from_dek(blob_id: BlobId, dek: Zeroizing<[u8; 32]>) -> Self;
    pub fn blob_id(&self) -> &BlobId;
    pub fn encrypt_chunk(&mut self, plaintext: &[u8], is_final: bool)
        -> Result<EncryptedChunk, AttachmentError>;
    pub fn finalize_reference(self) -> Result<BlobReference, AttachmentError>;
}

pub struct AttachmentDecryptor;
impl AttachmentDecryptor {
    pub fn new(blob_id: BlobId, dek: Zeroizing<[u8; 32]>) -> Self;
    pub fn from_reference(reference: &BlobReference) -> Self;
    pub fn decrypt_chunk(&mut self, chunk: &EncryptedChunk)
        -> Result<Vec<u8>, AttachmentError>;
    pub fn finish(&self) -> Result<u64, AttachmentError>;  // catches truncation
}

pub enum AttachmentError {
    InvalidParameter(&'static str),
    Misuse(&'static str),
    Aead,
    Truncated,
    AfterFinal,
    Csprng(String),
    TooLarge,
}
```

## Wire format

```text
EncryptedChunk {
    blob_id:    [u8; 16]
    index:      u32
    is_final:   bool
    ciphertext: Vec<u8>     ≤ 64 KiB
    tag:        [u8; 16]
}
```

AEAD primitive: XChaCha20-Poly1305.

Nonce per chunk:
`blob_id(16) || chunk_index_be(4) || 0x00 0x00 0x00 0x00`

AAD per chunk:
`"VAT1" || blob_id || chunk_index_be || is_final_byte || total_plaintext_len_be`

`is_final` and `chunk_index` are bound to the AAD, so a server
that flips either via metadata-only edit fails verification
(authenticated-encryption-with-associated-data semantics).

## Threat model & test coverage

| Threat                                                 | Defence                                          | Test                                     |
|--------------------------------------------------------|--------------------------------------------------|------------------------------------------|
| Storage flips a byte in a chunk                        | AEAD tag mismatch                                | `tampered_ciphertext_fails_aead`         |
| Storage flips the AEAD tag                             | AEAD verification                                | `tampered_tag_fails_aead`                |
| Server reorders chunks                                 | `chunk.index != decryptor.next_index`            | `reordered_chunks_rejected`              |
| Server swaps a chunk's index field                     | `chunk_index` in AAD                             | `forged_chunk_with_swapped_index_fails_aead` |
| Server promotes intermediate to final                  | `is_final` in AAD                                | `promoted_intermediate_to_final_rejected`|
| Server demotes final to intermediate                   | `is_final` in AAD                                | `demoted_final_to_intermediate_rejected` |
| Server drops the tail of a stream                      | `decryptor.finish()` returns `Truncated`         | `truncation_caught_by_finish`            |
| Server submits chunk after `is_final`                  | decryptor records final state; refuses           | `cannot_decrypt_after_final`             |
| Cross-blob mix (chunk from blob A → decryptor for B)   | `blob_id` in AAD                                 | `cross_blob_chunk_rejected`              |
| One blob's DEK leaks                                   | per-blob DEK from CSPRNG; others unaffected      | `new_random_yields_unique_ids_and_keys`  |
| `dbg!(blob_ref)` leaks DEK                             | hand-rolled redacted `Debug`                     | `blob_reference_debug_redacts_dek`       |
| `dbg!(chunk)` leaks ciphertext                         | hand-rolled redacted `Debug` (lengths only)      | `encrypted_chunk_debug_redacts_ciphertext` |
| Encryptor accepts oversized chunk                      | `plaintext.len() > CHUNK_SIZE` rejected          | `cannot_encrypt_oversize_chunk`          |
| Encryptor accepts empty intermediate chunk             | rejected (final may be empty)                    | `cannot_encrypt_empty_intermediate_chunk`|
| Encryptor reused after `is_final`                      | refused → `AfterFinal`                           | `cannot_encrypt_after_final`             |
| `finalize_reference` before final chunk                | refused → `Misuse`                               | `finalize_before_is_final_errors`        |
| Chunk-count overflow on encryption                     | `MAX_CHUNK_COUNT` cap; `checked_add`             | `chunk_count_overflow_protection`        |
| Peer encryptor exceeds chunk-count cap                  | decryptor mirrors `MAX_CHUNK_COUNT` check        | `decryptor_rejects_chunks_past_max_chunk_count` |
| Caller misuses `from_dek` with reused `(blob_id, dek)`  | doc comment elevated to a full "Caller responsibility — nonce uniqueness" block listing safe + unsafe patterns | structural / doc |
| `BlobReference::total_plaintext_len` is unauthenticated  | doc + caller verifies `dec.finish() == blob_ref.total_plaintext_len` | structural / doc |

## Bounds

`CHUNK_SIZE = 64 KiB`, `MAX_CHUNK_COUNT = 1 << 24`,
`MAX_PLAINTEXT_LEN = CHUNK_SIZE * MAX_CHUNK_COUNT` (~1 TiB),
`DEK_LEN = 32`, `TAG_LEN = 16`, `BLOB_ID_LEN = 16`.

## Out of scope (future PRs)

- Wire transport — VLT11.
- Multi-recipient DEK wrap — VLT04 wraps the DEK before
  persistence.
- Async streaming traits — the API is sync; an async wrapper
  is trivial.
- Resumable upload — naturally supported (chunks are
  independently AEAD-tagged).
- Content-deduplication — by design.
- Compression — host's responsibility (compress before
  `encrypt_chunk`).
- Forward-secure framing — future work.

## Citations

- VLT00-vault-roadmap.md — VLT14 placement.
- age v1 stream format — `crypto/stream` module of github.com/FiloSottile/age.
- HashiCorp Vault transit-engine framing.
- VLT01-vault-sealed-store — record-level sealing companion.
- VLT04-vault-recipients — wraps the DEK for multi-recipient.
- VLT10-vault-sync-engine — propagates chunks across devices.
- `coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_*` —
  AEAD primitive.
