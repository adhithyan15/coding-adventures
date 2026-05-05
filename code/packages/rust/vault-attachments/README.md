# `coding_adventures_vault_attachments` — VLT14

Streamable encrypted blob attachments. Big-by-vault-standards
files (multi-MiB photos, exports, document scans) split into
fixed-size 64 KiB chunks, each AEAD-encrypted under a per-blob
DEK with counter-derived nonces. Borrows from age v1's stream
format and HashiCorp Vault's transit framing.

## Quick example

```rust
use coding_adventures_vault_attachments::{
    AttachmentDecryptor, AttachmentEncryptor, BlobReference, CHUNK_SIZE,
};

// --- Encrypt ---
let mut enc = AttachmentEncryptor::new_random()?;
let mut buf = vec![0u8; CHUNK_SIZE];
loop {
    let n = read_from_disk(&mut buf)?;
    let is_final = n < CHUNK_SIZE;
    let chunk = enc.encrypt_chunk(&buf[..n], is_final)?;
    persist(&chunk)?;       // store as a sync record
    if is_final { break; }
}
let blob_ref: BlobReference = enc.finalize_reference()?;
// Wrap blob_ref.dek with VLT04 BEFORE storage; never persist
// the DEK in plaintext.

// --- Decrypt (later) ---
let mut dec = AttachmentDecryptor::from_reference(&blob_ref);
for chunk in load_chunks(&blob_ref.blob_id)? {
    let plaintext = dec.decrypt_chunk(&chunk)?;
    consume(plaintext);
}
let total = dec.finish()?;  // catches truncation
```

## Wire format per chunk

```text
EncryptedChunk {
    blob_id:   [u8; 16],
    index:     u32,
    is_final:  bool,
    ciphertext: Vec<u8>,    // <= 64 KiB, XChaCha20 stream
    tag:       [u8; 16],    // Poly1305
}
```

AEAD nonce: `blob_id(16) || chunk_index_be(4) || 0x00 0x00 0x00 0x00`.
AEAD AAD: `"VAT1" || blob_id || chunk_index_be || is_final_byte || total_plaintext_len_be`.

Both the chunk index and the `is_final` flag are bound to the
AAD, so a server cannot reorder chunks or "promote" an
intermediate chunk into the final position via metadata edit.
`blob_id` is in the AAD so a chunk from blob A submitted to a
decryptor for blob B fails verification rather than producing
a silent crossed-stream.

## Threat model

| Attack                                              | Caught by                                  |
|----------------------------------------------------|--------------------------------------------|
| Storage flips a byte in a chunk                    | AEAD tag mismatch → `Aead`                 |
| Chunks reordered                                    | `chunk.index != decryptor.next_index` → `Misuse` |
| Chunks deleted (truncation)                         | `decryptor.finish()` returns `Truncated`   |
| Chunk from blob A submitted to decryptor B          | `blob_id` in AAD → `Aead`                  |
| Server promotes intermediate to final               | `is_final` in AAD → `Aead`                 |
| Server demotes final to intermediate                | `is_final` in AAD → `Aead`                 |
| One blob's DEK leaks                                | other blobs use distinct CSPRNG-drawn DEKs |
| `dbg!(blob_ref)` leaks DEK                          | hand-rolled redacted `Debug` on `BlobReference` |
| `dbg!(chunk)` leaks ciphertext                      | hand-rolled redacted `Debug` on `EncryptedChunk` |
| Chunk count overflow                                | `MAX_CHUNK_COUNT` cap; `checked_add`       |

## Bounds

| Constant                | Value         |
|-------------------------|---------------|
| `CHUNK_SIZE`            | 64 KiB        |
| `MAX_CHUNK_COUNT`       | 16,777,216    |
| `MAX_PLAINTEXT_LEN`     | ~1 TiB        |
| `DEK_LEN`               | 32 bytes      |
| `TAG_LEN`               | 16 bytes      |
| `BLOB_ID_LEN`           | 16 bytes      |

## What this crate is NOT

- Not a network layer — VLT11 transports carry the chunks.
- Not a recipient-list — VLT04 wrap-set wraps the DEK for
  multi-recipient sharing.
- Not a sync layer — VLT10 propagates chunks like any other
  ciphertext record.
- Not a deduper — same plaintext blob uploaded twice gets two
  distinct blob ids and two distinct DEKs.

## Capabilities

`csprng` (for blob id + DEK in `new_random`). See
`required_capabilities.json`.

See [`VLT00-vault-roadmap.md`](../../../specs/VLT00-vault-roadmap.md)
and [`VLT14-vault-attachments.md`](../../../specs/VLT14-vault-attachments.md).
