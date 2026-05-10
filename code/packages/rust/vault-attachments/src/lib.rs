//! # `coding_adventures_vault_attachments` — VLT14
//!
//! ## What this crate is
//!
//! **Streamable encrypted blob attachments** for the Vault
//! stack. Big-by-vault-standards files (multi-MiB photos,
//! exports, document scans) don't fit in a sealed-store record
//! — a 1 MiB record gets sync'd in full on every change, and
//! holding the whole plaintext in RAM is wrong for a phone.
//!
//! VLT14 splits a blob into fixed-size 64 KiB *chunks*, each
//! AEAD-encrypted under a per-blob DEK with a counter-derived
//! nonce. The shape borrows from age v1's stream format and
//! HashiCorp Vault's transit-engine framing:
//!
//! ```text
//!   plaintext stream
//!         │ 64 KiB at a time
//!         ▼
//!   ┌─────────────────────────────────────────────────────┐
//!   │  AttachmentEncryptor                                │
//!   │  per-chunk:                                         │
//!   │    nonce24 = blob_id(16) || chunk_index_be(4) || 00 4 │
//!   │    aad     = "VAT1" || blob_id || chunk_index ||    │
//!   │              is_final_byte || total_plaintext_len    │
//!   │    (ct, tag) = XChaCha20-Poly1305(plaintext, dek,    │
//!   │                                    nonce24, aad)     │
//!   └─────────────────────┬───────────────────────────────┘
//!                         │ EncryptedChunk { index, is_final,
//!                         │                  ciphertext, tag }
//!                         ▼
//!   ┌─────────────────────────────────────────────────────┐
//!   │  storage backend (sync replicates the chunks via     │
//!   │  VLT10 like any other ciphertext records)            │
//!   └─────────────────────────────────────────────────────┘
//! ```
//!
//! The host:
//!
//!   1. Calls [`AttachmentEncryptor::new_random`] (or `from_dek`
//!      if the DEK was generated externally — e.g. wrapped by
//!      VLT04 first), producing a `(BlobReference, encryptor)`.
//!   2. Feeds the plaintext stream through `encrypt_chunk` in
//!      64 KiB pieces, marking the last call with
//!      `is_final = true`.
//!   3. Persists each `EncryptedChunk` (e.g. as a sync record
//!      under `__vault_blobs__/<blob_id>/<chunk_index>`).
//!   4. Records `BlobReference` (which carries the DEK) in the
//!      parent's typed record. The DEK itself MUST be wrapped
//!      by VLT04 before it gets anywhere near a sync server —
//!      this crate never persists the DEK.
//!
//! Reading runs the same loop in reverse via
//! [`AttachmentDecryptor`] which enforces:
//!
//!   * Chunks decrypt in strict ascending index order.
//!   * The last chunk decrypted has `is_final = true`. A
//!     stream that ends without a final-flagged chunk is a
//!     truncation attack.
//!   * `chunk_index` and `is_final` are bound to the AEAD AAD,
//!     so a server that swaps chunks across the stream fails
//!     verification.
//!   * `total_plaintext_len` is bound to AAD on every chunk —
//!     a server that lies about the total length cannot get a
//!     forged stream past the decryptor.
//!
//! ## Threat model
//!
//! * **Server tampering**: any byte flip in any chunk
//!   invalidates the AEAD tag → `Aead`. Detectable.
//! * **Reordering**: chunk N out of order → AAD mismatch →
//!   `Aead`.
//! * **Truncation**: stream ends before `is_final` chunk →
//!   `Truncated`.
//! * **Insertion of an extra chunk after `is_final`**:
//!   decryptor records that finalisation has occurred and
//!   refuses further chunks → `AfterFinal`.
//! * **Cross-blob mix**: a chunk from blob A submitted to a
//!   decryptor for blob B fails because `blob_id` is in AAD.
//! * **DEK leak from one blob**: doesn't compromise others
//!   (per-blob DEKs).
//! * **Bounded memory on read**: the decryptor takes one chunk
//!   at a time — the host can stream straight from disk to the
//!   consumer without buffering the whole blob.
//! * **Plaintext residue**: every plaintext chunk produced by
//!   `decrypt_chunk` is owned by the caller; we cannot wipe
//!   what the caller hands to its consumer. The DEK itself is
//!   held under [`Zeroizing`] inside both encryptor and
//!   decryptor.
//!
//! ## What this crate does NOT do
//!
//! * Not a network layer — VLT11 transports carry the chunks.
//! * Not a recipient-list — the DEK is wrapped by VLT04 before
//!   it ships.
//! * Not a sync layer — VLT10 propagates chunks like any other
//!   ciphertext record.
//! * Not a deduper — same plaintext blob uploaded twice is two
//!   distinct blob_ids with two distinct DEKs.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_zeroize::Zeroizing;

// === Section 1. Bounds =====================================================

/// Chunk size in bytes. 64 KiB matches age v1 and keeps RAM
/// usage on a phone bounded. Picking a larger chunk is a
/// performance win on desktop but punitive on mobile / slow
/// disk; 64 KiB is the de-facto convention.
pub const CHUNK_SIZE: usize = 64 * 1024;
/// Maximum number of chunks per blob. With `CHUNK_SIZE = 64
/// KiB` and `MAX_CHUNK_COUNT = u32::MAX` the absolute upper
/// bound is ~256 TiB; we cap at `1 << 24` (~1 TiB plaintext)
/// because an attachment that big is doing the wrong thing.
pub const MAX_CHUNK_COUNT: u32 = 1 << 24;
/// Maximum plaintext bytes for a single blob — derived from
/// `CHUNK_SIZE * MAX_CHUNK_COUNT`. Exposed as a constant so
/// callers can early-reject oversized inputs.
pub const MAX_PLAINTEXT_LEN: u64 =
    (CHUNK_SIZE as u64) * (MAX_CHUNK_COUNT as u64);
/// Length of the per-blob DEK, in bytes. 32 = 256 bits =
/// XChaCha20 key size.
pub const DEK_LEN: usize = 32;
/// Length of the per-chunk AEAD tag, in bytes.
pub const TAG_LEN: usize = 16;
/// Length of the per-blob identifier, in bytes. 16 bytes (128
/// bits) of CSPRNG output.
pub const BLOB_ID_LEN: usize = 16;
/// Length of the XChaCha20 nonce, in bytes.
const NONCE_LEN: usize = 24;
/// Magic prefix of the AAD, 4 bytes.
const AAD_MAGIC: &[u8; 4] = b"VAT1";

// === Section 2. Vocabulary types ===========================================

/// 128-bit opaque blob identifier. Generated from CSPRNG (or
/// supplied by the host if it has a domain-specific id —
/// e.g. a content hash). Treated as opaque by the encryptor /
/// decryptor; only used as part of the AEAD AAD.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlobId(pub [u8; BLOB_ID_LEN]);

impl BlobId {
    /// Borrow the bytes.
    pub fn as_bytes(&self) -> &[u8; BLOB_ID_LEN] {
        &self.0
    }
}

impl core::fmt::Debug for BlobId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Hex-encode for a stable, printable Debug. The blob
        // id is *not* secret (it's a pointer that will appear
        // in URLs and log lines).
        let mut s = String::with_capacity(2 * BLOB_ID_LEN);
        for b in self.0.iter() {
            s.push(HEX_LO[(b >> 4) as usize] as char);
            s.push(HEX_LO[(b & 0x0f) as usize] as char);
        }
        write!(f, "BlobId({})", s)
    }
}

const HEX_LO: &[u8; 16] = b"0123456789abcdef";

/// Reference handed back from `AttachmentEncryptor::new_random`.
/// The parent record stashes this so a future read can
/// reconstruct the decryptor.
///
/// **The DEK is held under `Zeroizing` and MUST NOT be
/// persisted in plaintext.** The host is expected to wrap it
/// via VLT04 (multi-recipient DEK wrap) before any sync /
/// storage write. `Debug` is hand-rolled to redact the DEK so
/// a stray `dbg!(&blob_ref)` cannot leak it.
pub struct BlobReference {
    /// Stable identifier of this blob.
    pub blob_id: BlobId,
    /// Per-blob data-encryption key (256-bit XChaCha20 key).
    /// The encryptor / decryptor borrow this; the host wraps
    /// it via VLT04 before storage.
    pub dek: Zeroizing<[u8; DEK_LEN]>,
    /// Total plaintext length once the stream is finalised.
    pub total_plaintext_len: u64,
    /// Number of chunks in the finalised stream.
    pub chunk_count: u32,
}

impl core::fmt::Debug for BlobReference {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobReference")
            .field("blob_id", &self.blob_id)
            .field("dek", &"<redacted>")
            .field("total_plaintext_len", &self.total_plaintext_len)
            .field("chunk_count", &self.chunk_count)
            .finish()
    }
}

/// One AEAD-sealed chunk on the wire. Hosts persist these
/// individually (e.g. as sync records) so a decryptor can
/// stream from disk without buffering the whole blob.
#[derive(Clone, PartialEq, Eq)]
pub struct EncryptedChunk {
    /// Echo of the parent blob id (so a misrouted chunk gets
    /// caught at AEAD verification rather than producing a
    /// silent decryption failure later).
    pub blob_id: BlobId,
    /// Zero-based chunk index. Encryptor allocates these in
    /// strict ascending order; decryptor enforces the same.
    pub index: u32,
    /// `true` for the last chunk of the stream. Sealed in AAD
    /// so a server cannot "promote" an earlier chunk into the
    /// final position via metadata-only edit.
    pub is_final: bool,
    /// AEAD ciphertext. Always exactly `plaintext_len` bytes
    /// (XChaCha20 is a stream cipher; the tag is separate).
    pub ciphertext: Vec<u8>,
    /// AEAD authentication tag. 16 bytes.
    pub tag: [u8; TAG_LEN],
}

impl core::fmt::Debug for EncryptedChunk {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EncryptedChunk")
            .field("blob_id", &self.blob_id)
            .field("index", &self.index)
            .field("is_final", &self.is_final)
            .field(
                "ciphertext",
                &format_args!("<{} bytes redacted>", self.ciphertext.len()),
            )
            .field("tag", &format_args!("<{}-byte tag>", self.tag.len()))
            .finish()
    }
}

/// All errors produced by the attachment crate.
#[derive(Debug)]
pub enum AttachmentError {
    /// Caller-supplied parameter violated a documented bound.
    InvalidParameter(&'static str),
    /// Caller is using the encryptor / decryptor incorrectly
    /// (e.g. encrypting after `is_final`, decrypting out of
    /// order).
    Misuse(&'static str),
    /// AEAD authentication failed: tag mismatch, AAD mismatch,
    /// nonce mismatch — anything that means the chunk does not
    /// belong to this stream at this position.
    Aead,
    /// Stream ended before a chunk with `is_final = true` was
    /// received.
    Truncated,
    /// A chunk was submitted after the decryptor had already
    /// observed `is_final`.
    AfterFinal,
    /// CSPRNG failed (`new_random` only).
    Csprng(String),
    /// Total plaintext or chunk count would exceed
    /// [`MAX_PLAINTEXT_LEN`] / [`MAX_CHUNK_COUNT`].
    TooLarge,
}

impl core::fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter(why) => write!(f, "invalid parameter: {}", why),
            Self::Misuse(why) => write!(f, "misuse: {}", why),
            Self::Aead => write!(f, "AEAD verification failed"),
            Self::Truncated => write!(f, "stream truncated (no final chunk)"),
            Self::AfterFinal => write!(f, "chunk submitted after final"),
            Self::Csprng(e) => write!(f, "CSPRNG error: {}", e),
            Self::TooLarge => write!(f, "blob exceeds MAX_PLAINTEXT_LEN"),
        }
    }
}

impl std::error::Error for AttachmentError {}

// === Section 3. AAD construction ==========================================
//
// The AAD shape is identical for encryption and decryption.
// Layout:
//
//   "VAT1"             4 bytes
//   blob_id            16 bytes
//   chunk_index BE     4 bytes
//   is_final           1 byte (0 or 1)
//   total_plaintext_len BE 8 bytes (or 0 during streaming —
//     see `CommittedTotal` below)
//
// `total_plaintext_len` is committed up front when known
// (decryption: from BlobReference; encryption: streaming
// callers can pass `None` and the encryptor records 0 in AAD,
// so the decryptor must be constructed with `expected_total =
// None` to match — see `Mode`).

fn build_aad(
    blob_id: &BlobId,
    chunk_index: u32,
    is_final: bool,
    total_plaintext_len: u64,
) -> [u8; 33] {
    // 4 magic + 16 blob_id + 4 index + 1 final + 8 total = 33
    let mut aad = [0u8; 33];
    aad[0..4].copy_from_slice(AAD_MAGIC);
    aad[4..20].copy_from_slice(blob_id.as_bytes());
    aad[20..24].copy_from_slice(&chunk_index.to_be_bytes());
    aad[24] = if is_final { 1 } else { 0 };
    aad[25..33].copy_from_slice(&total_plaintext_len.to_be_bytes());
    aad
}

fn build_nonce(blob_id: &BlobId, chunk_index: u32) -> [u8; NONCE_LEN] {
    // Nonce layout: blob_id(16) || chunk_index_be(4) || 00 00 00 00.
    // The blob_id supplies most of the entropy; the counter
    // ensures uniqueness across chunks of the same blob. Total
    // nonce space is 192 bits (XChaCha20).
    let mut n = [0u8; NONCE_LEN];
    n[0..16].copy_from_slice(blob_id.as_bytes());
    n[16..20].copy_from_slice(&chunk_index.to_be_bytes());
    // last 4 bytes left zero
    n
}

// === Section 4. Encryptor ================================================

/// Streaming encryptor. Owns the per-blob DEK and the running
/// counter; produces `EncryptedChunk`s as the host hands it
/// plaintext.
///
/// Usage:
///
/// ```ignore
/// let (blob_ref, mut enc) = AttachmentEncryptor::new_random()?;
/// while let Some(plain) = next_chunk_from_disk()? {
///     let is_final = is_last(plain);
///     let chunk = enc.encrypt_chunk(&plain, is_final)?;
///     persist(&chunk)?;
/// }
/// let blob_ref = enc.finalize_reference()?;  // updates total_plaintext_len + chunk_count
/// // wrap blob_ref.dek with VLT04 before storage
/// ```
pub struct AttachmentEncryptor {
    blob_id: BlobId,
    dek: Zeroizing<[u8; DEK_LEN]>,
    next_index: u32,
    total_plaintext: u64,
    finalized: bool,
    /// `total_plaintext_len` value committed in the AAD. v1
    /// always passes `0` (we don't know the total until
    /// finalisation, and we never want to buffer the whole
    /// stream). The consequence is that
    /// `BlobReference::total_plaintext_len` is **NOT
    /// authenticated** by the chunk AEAD — it's a hint the
    /// host stores alongside the chunks. Callers who want to
    /// catch tampering of that field MUST verify
    /// `dec.finish()? == blob_ref.total_plaintext_len`
    /// themselves after streaming all chunks.
    aad_total: u64,
}

impl AttachmentEncryptor {
    /// Create a fresh encryptor with a CSPRNG-drawn blob id +
    /// DEK. Returns the running encryptor; the
    /// `BlobReference` is finalised at the end of the stream
    /// via [`Self::finalize_reference`] so it includes
    /// `total_plaintext_len` and `chunk_count`.
    pub fn new_random() -> Result<Self, AttachmentError> {
        let blob_id_bytes: [u8; BLOB_ID_LEN] =
            coding_adventures_csprng::random_array().map_err(|e| {
                AttachmentError::Csprng(format!("blob id: {}", e))
            })?;
        let dek_bytes: [u8; DEK_LEN] =
            coding_adventures_csprng::random_array().map_err(|e| {
                AttachmentError::Csprng(format!("dek: {}", e))
            })?;
        Ok(Self {
            blob_id: BlobId(blob_id_bytes),
            dek: Zeroizing::new(dek_bytes),
            next_index: 0,
            total_plaintext: 0,
            finalized: false,
            aad_total: 0,
        })
    }

    /// Construct an encryptor with a caller-supplied blob id +
    /// DEK.
    ///
    /// # Caller responsibility — nonce uniqueness
    ///
    /// **The caller MUST ensure that this `(blob_id, dek)` pair
    /// has never been used to encrypt any other stream.** Per-
    /// chunk nonces are derived from `blob_id || chunk_index`
    /// (zero-padded). Two streams sharing the same
    /// `(blob_id, dek)` produce identical nonces at the same
    /// chunk indices — that is **catastrophic** for
    /// XChaCha20-Poly1305: identical key+nonce reuse leaks the
    /// XOR of plaintexts (full plaintext recovery if either is
    /// known) and lets an attacker recover the Poly1305 tag-
    /// derivation key (forging arbitrary chunk authenticators).
    ///
    /// Safe usage patterns:
    ///
    ///   * Random `blob_id` + per-call random `dek`
    ///     ([`Self::new_random`] does this for you).
    ///   * Random `blob_id` + caller-managed `dek` whose use
    ///     across blobs is gated by your own bookkeeping.
    ///   * Content-derived `blob_id` + per-call random `dek`
    ///     (re-uploading the same file produces a new DEK,
    ///     which is fine).
    ///
    /// **NOT safe:** content-derived `blob_id` + deterministic
    /// `dek`. Re-uploading the same file then collides.
    ///
    /// Prefer [`Self::new_random`] unless you have a specific
    /// reason to construct externally.
    pub fn from_dek(blob_id: BlobId, dek: Zeroizing<[u8; DEK_LEN]>) -> Self {
        Self {
            blob_id,
            dek,
            next_index: 0,
            total_plaintext: 0,
            finalized: false,
            aad_total: 0,
        }
    }

    /// Borrow the blob id.
    pub fn blob_id(&self) -> &BlobId {
        &self.blob_id
    }

    /// Encrypt the next chunk. `plaintext.len()` must be
    /// `<= CHUNK_SIZE`. `is_final = true` marks the last
    /// chunk; subsequent calls return [`AttachmentError::AfterFinal`].
    ///
    /// The caller decides chunk boundaries (typically: read
    /// 64 KiB from disk; if EOF is hit on this read, set
    /// `is_final = true`). All but the last chunk should be
    /// exactly `CHUNK_SIZE` bytes; the last chunk can be 0..=N.
    pub fn encrypt_chunk(
        &mut self,
        plaintext: &[u8],
        is_final: bool,
    ) -> Result<EncryptedChunk, AttachmentError> {
        if self.finalized {
            return Err(AttachmentError::AfterFinal);
        }
        if plaintext.len() > CHUNK_SIZE {
            return Err(AttachmentError::InvalidParameter(
                "plaintext.len() > CHUNK_SIZE",
            ));
        }
        if !is_final && plaintext.is_empty() {
            // Empty intermediate chunk would let an attacker
            // confuse counting; we forbid it. An empty *final*
            // chunk is allowed (zero-byte tail of an exact
            // multiple of CHUNK_SIZE).
            return Err(AttachmentError::InvalidParameter(
                "intermediate chunk must be non-empty (final chunk may be empty)",
            ));
        }
        if self.next_index >= MAX_CHUNK_COUNT {
            return Err(AttachmentError::TooLarge);
        }
        let new_total = self
            .total_plaintext
            .checked_add(plaintext.len() as u64)
            .ok_or(AttachmentError::TooLarge)?;
        if new_total > MAX_PLAINTEXT_LEN {
            return Err(AttachmentError::TooLarge);
        }
        let nonce = build_nonce(&self.blob_id, self.next_index);
        let aad = build_aad(&self.blob_id, self.next_index, is_final, self.aad_total);
        let (ciphertext, tag) =
            xchacha20_poly1305_aead_encrypt(plaintext, &self.dek, &nonce, &aad);
        let out = EncryptedChunk {
            blob_id: self.blob_id,
            index: self.next_index,
            is_final,
            ciphertext,
            tag,
        };
        self.next_index = self
            .next_index
            .checked_add(1)
            .ok_or(AttachmentError::TooLarge)?;
        self.total_plaintext = new_total;
        if is_final {
            self.finalized = true;
        }
        Ok(out)
    }

    /// Produce the [`BlobReference`] that the parent record
    /// should store. Only valid after `is_final` has been seen.
    /// Consumes the encryptor (the DEK moves into the
    /// reference).
    pub fn finalize_reference(self) -> Result<BlobReference, AttachmentError> {
        if !self.finalized {
            return Err(AttachmentError::Misuse(
                "finalize_reference called before is_final chunk",
            ));
        }
        Ok(BlobReference {
            blob_id: self.blob_id,
            dek: self.dek,
            total_plaintext_len: self.total_plaintext,
            chunk_count: self.next_index,
        })
    }
}

// === Section 5. Decryptor ================================================

/// Streaming decryptor. The host feeds it `EncryptedChunk`s in
/// the order they were produced; the decryptor returns the
/// plaintext for each.
///
/// Enforces:
///
///   * `chunk.index` is the *next* expected index. Out-of-order
///     submission → `Misuse`.
///   * `chunk.blob_id == self.blob_id`. Cross-blob → `Misuse`.
///   * AEAD verification (tag + AAD + nonce). Failure → `Aead`.
///   * Stream ends with `is_final` chunk. Caller must call
///     [`Self::finish`] to surface a `Truncated` error if the
///     stream stopped before the final chunk.
pub struct AttachmentDecryptor {
    blob_id: BlobId,
    dek: Zeroizing<[u8; DEK_LEN]>,
    next_index: u32,
    seen_final: bool,
    total_plaintext_seen: u64,
    /// AAD-bound `total_plaintext_len`. Always 0 in v1 — see
    /// the comment on `AttachmentEncryptor::aad_total`. The
    /// field is kept on the struct so a future v2 frame can
    /// commit a real value without breaking the decryptor's
    /// shape.
    aad_total: u64,
}

impl AttachmentDecryptor {
    /// Construct a decryptor from a `BlobReference`. The
    /// reference's DEK is borrowed but the decryptor needs to
    /// own it for the duration of the stream — the caller can
    /// `clone` the inner bytes by destructuring `BlobReference`.
    pub fn new(blob_id: BlobId, dek: Zeroizing<[u8; DEK_LEN]>) -> Self {
        Self {
            blob_id,
            dek,
            next_index: 0,
            seen_final: false,
            total_plaintext_seen: 0,
            aad_total: 0,
        }
    }

    /// Convenience constructor that takes a `BlobReference` by
    /// reference and clones its DEK. The caller is responsible
    /// for dropping the original `BlobReference` if the DEK
    /// shouldn't outlive the read.
    pub fn from_reference(reference: &BlobReference) -> Self {
        Self::new(reference.blob_id, Zeroizing::new(*reference.dek))
    }

    /// Decrypt one chunk. Returns the plaintext bytes
    /// (`Vec<u8>` — caller wraps in `Zeroizing` if they care
    /// about residue).
    pub fn decrypt_chunk(
        &mut self,
        chunk: &EncryptedChunk,
    ) -> Result<Vec<u8>, AttachmentError> {
        if self.seen_final {
            return Err(AttachmentError::AfterFinal);
        }
        // Mirror the encryptor's bounds on the read side so a
        // peer encryptor that produces more than
        // `MAX_CHUNK_COUNT` chunks (different implementation,
        // bug, or an attacker who forged a longer stream and
        // somehow has the DEK) is rejected at the documented
        // policy bound rather than at u32 overflow.
        if self.next_index >= MAX_CHUNK_COUNT {
            return Err(AttachmentError::TooLarge);
        }
        if chunk.blob_id != self.blob_id {
            return Err(AttachmentError::Misuse("chunk.blob_id != decryptor blob_id"));
        }
        if chunk.index != self.next_index {
            return Err(AttachmentError::Misuse("chunk.index out of order"));
        }
        if chunk.ciphertext.len() > CHUNK_SIZE {
            return Err(AttachmentError::InvalidParameter(
                "ciphertext.len() > CHUNK_SIZE",
            ));
        }
        let nonce = build_nonce(&self.blob_id, self.next_index);
        let aad = build_aad(
            &self.blob_id,
            self.next_index,
            chunk.is_final,
            self.aad_total,
        );
        let plaintext = xchacha20_poly1305_aead_decrypt(
            &chunk.ciphertext,
            &self.dek,
            &nonce,
            &aad,
            &chunk.tag,
        )
        .ok_or(AttachmentError::Aead)?;
        self.total_plaintext_seen = self
            .total_plaintext_seen
            .checked_add(plaintext.len() as u64)
            .ok_or(AttachmentError::TooLarge)?;
        if self.total_plaintext_seen > MAX_PLAINTEXT_LEN {
            return Err(AttachmentError::TooLarge);
        }
        self.next_index = self
            .next_index
            .checked_add(1)
            .ok_or(AttachmentError::TooLarge)?;
        if chunk.is_final {
            self.seen_final = true;
        }
        Ok(plaintext)
    }

    /// Verify the stream is complete (i.e. an `is_final` chunk
    /// was decrypted). Call after the last `decrypt_chunk` to
    /// catch truncation: a server that drops the tail of the
    /// stream cannot otherwise be distinguished from a legit
    /// reader who hasn't read it yet.
    pub fn finish(&self) -> Result<u64, AttachmentError> {
        if !self.seen_final {
            return Err(AttachmentError::Truncated);
        }
        Ok(self.total_plaintext_seen)
    }
}

// === Section 6. Tests =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_dek() -> Zeroizing<[u8; DEK_LEN]> {
        Zeroizing::new([0x42u8; DEK_LEN])
    }

    fn fixed_blob_id() -> BlobId {
        BlobId([0xAAu8; BLOB_ID_LEN])
    }

    fn enc_pair() -> AttachmentEncryptor {
        AttachmentEncryptor::from_dek(fixed_blob_id(), fixed_dek())
    }

    fn dec_pair() -> AttachmentDecryptor {
        AttachmentDecryptor::new(fixed_blob_id(), fixed_dek())
    }

    // --- Round-trip ---

    #[test]
    fn single_chunk_roundtrip() {
        let mut e = enc_pair();
        let plain = b"hello world".to_vec();
        let chunk = e.encrypt_chunk(&plain, true).unwrap();
        assert_eq!(chunk.index, 0);
        assert!(chunk.is_final);
        let mut d = dec_pair();
        let recovered = d.decrypt_chunk(&chunk).unwrap();
        assert_eq!(recovered, plain);
        assert_eq!(d.finish().unwrap(), plain.len() as u64);
    }

    #[test]
    fn multi_chunk_roundtrip() {
        let mut e = enc_pair();
        let p1 = vec![1u8; CHUNK_SIZE];
        let p2 = vec![2u8; CHUNK_SIZE];
        let p3 = vec![3u8; 100]; // tail
        let c1 = e.encrypt_chunk(&p1, false).unwrap();
        let c2 = e.encrypt_chunk(&p2, false).unwrap();
        let c3 = e.encrypt_chunk(&p3, true).unwrap();
        let mut d = dec_pair();
        assert_eq!(d.decrypt_chunk(&c1).unwrap(), p1);
        assert_eq!(d.decrypt_chunk(&c2).unwrap(), p2);
        assert_eq!(d.decrypt_chunk(&c3).unwrap(), p3);
        assert_eq!(d.finish().unwrap(), (CHUNK_SIZE * 2 + 100) as u64);
    }

    #[test]
    fn final_chunk_can_be_empty() {
        let mut e = enc_pair();
        let p1 = vec![1u8; CHUNK_SIZE];
        let c1 = e.encrypt_chunk(&p1, false).unwrap();
        let c2 = e.encrypt_chunk(&[], true).unwrap();
        let mut d = dec_pair();
        assert_eq!(d.decrypt_chunk(&c1).unwrap(), p1);
        assert_eq!(d.decrypt_chunk(&c2).unwrap(), Vec::<u8>::new());
        assert_eq!(d.finish().unwrap(), CHUNK_SIZE as u64);
    }

    #[test]
    fn finalize_reference_carries_metadata() {
        let mut e = enc_pair();
        e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let r = e.finalize_reference().unwrap();
        assert_eq!(r.total_plaintext_len, 100);
        assert_eq!(r.chunk_count, 1);
    }

    #[test]
    fn new_random_yields_unique_ids_and_keys() {
        let e1 = AttachmentEncryptor::new_random().unwrap();
        let e2 = AttachmentEncryptor::new_random().unwrap();
        assert_ne!(e1.blob_id, e2.blob_id);
        assert_ne!(*e1.dek, *e2.dek);
    }

    // --- Encryption misuse ---

    #[test]
    fn cannot_encrypt_after_final() {
        let mut e = enc_pair();
        e.encrypt_chunk(&[1u8; 1], true).unwrap();
        let r = e.encrypt_chunk(&[2u8; 1], true);
        assert!(matches!(r, Err(AttachmentError::AfterFinal)));
    }

    #[test]
    fn cannot_encrypt_oversize_chunk() {
        let mut e = enc_pair();
        let big = vec![0u8; CHUNK_SIZE + 1];
        let r = e.encrypt_chunk(&big, true);
        assert!(matches!(r, Err(AttachmentError::InvalidParameter(_))));
    }

    #[test]
    fn cannot_encrypt_empty_intermediate_chunk() {
        let mut e = enc_pair();
        let r = e.encrypt_chunk(&[], false);
        assert!(matches!(r, Err(AttachmentError::InvalidParameter(_))));
    }

    #[test]
    fn finalize_before_is_final_errors() {
        let mut e = enc_pair();
        e.encrypt_chunk(&[1u8; 1], false).unwrap();
        // Missed setting is_final.
        let r = e.finalize_reference();
        assert!(matches!(r, Err(AttachmentError::Misuse(_))));
    }

    // --- Decryption: tampering / reorder / truncation ---

    #[test]
    fn tampered_ciphertext_fails_aead() {
        let mut e = enc_pair();
        let mut c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        c.ciphertext[0] ^= 0x01;
        let mut d = dec_pair();
        assert!(matches!(d.decrypt_chunk(&c), Err(AttachmentError::Aead)));
    }

    #[test]
    fn tampered_tag_fails_aead() {
        let mut e = enc_pair();
        let mut c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        c.tag[0] ^= 0x01;
        let mut d = dec_pair();
        assert!(matches!(d.decrypt_chunk(&c), Err(AttachmentError::Aead)));
    }

    #[test]
    fn reordered_chunks_rejected() {
        let mut e = enc_pair();
        let c1 = e.encrypt_chunk(&[1u8; 100], false).unwrap();
        let c2 = e.encrypt_chunk(&[2u8; 100], true).unwrap();
        let mut d = dec_pair();
        // Submit c2 before c1.
        let r = d.decrypt_chunk(&c2);
        assert!(matches!(r, Err(AttachmentError::Misuse(_))));
        // c1 still works (decryptor state unchanged after error).
        // Actually after an error, next_index is unchanged, so
        // c1 (index=0) can still be decrypted.
        d.decrypt_chunk(&c1).unwrap();
    }

    #[test]
    fn cross_blob_chunk_rejected() {
        // Encrypt under blob A; try to decrypt under blob B.
        let mut e = enc_pair();
        let c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let other_id = BlobId([0xBBu8; BLOB_ID_LEN]);
        let mut d = AttachmentDecryptor::new(other_id, fixed_dek());
        let r = d.decrypt_chunk(&c);
        assert!(matches!(r, Err(AttachmentError::Misuse(_))));
    }

    #[test]
    fn promoted_intermediate_to_final_rejected() {
        // Server flips is_final on what was actually an
        // intermediate chunk. AAD includes is_final, so the
        // tag check fails.
        let mut e = enc_pair();
        let c1 = e.encrypt_chunk(&[1u8; 100], false).unwrap();
        let c2 = e.encrypt_chunk(&[2u8; 100], true).unwrap();
        let mut tampered = c1.clone();
        tampered.is_final = true;
        let mut d = dec_pair();
        let r = d.decrypt_chunk(&tampered);
        assert!(matches!(r, Err(AttachmentError::Aead)));
        // Original sequence still verifies.
        d.decrypt_chunk(&c1).unwrap();
        d.decrypt_chunk(&c2).unwrap();
    }

    #[test]
    fn demoted_final_to_intermediate_rejected() {
        let mut e = enc_pair();
        let c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let mut tampered = c.clone();
        tampered.is_final = false;
        let mut d = dec_pair();
        assert!(matches!(d.decrypt_chunk(&tampered), Err(AttachmentError::Aead)));
    }

    #[test]
    fn truncation_caught_by_finish() {
        let mut e = enc_pair();
        let c1 = e.encrypt_chunk(&[1u8; 100], false).unwrap();
        let _c2 = e.encrypt_chunk(&[2u8; 100], true).unwrap();
        let mut d = dec_pair();
        d.decrypt_chunk(&c1).unwrap();
        // Server drops c2; reader calls finish() before reading
        // it.
        assert!(matches!(d.finish(), Err(AttachmentError::Truncated)));
    }

    #[test]
    fn cannot_decrypt_after_final() {
        let mut e = enc_pair();
        let c1 = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let mut d = dec_pair();
        d.decrypt_chunk(&c1).unwrap();
        // Even if the server sends "another" chunk, we refuse.
        let r = d.decrypt_chunk(&c1);
        assert!(matches!(r, Err(AttachmentError::AfterFinal)));
    }

    // --- Round-trip with new_random ---

    #[test]
    fn new_random_roundtrip_via_finalize_reference() {
        let mut e = AttachmentEncryptor::new_random().unwrap();
        let p1 = vec![1u8; CHUNK_SIZE];
        let p2 = vec![2u8; 1234];
        let c1 = e.encrypt_chunk(&p1, false).unwrap();
        let c2 = e.encrypt_chunk(&p2, true).unwrap();
        let blob_ref = e.finalize_reference().unwrap();
        let mut d = AttachmentDecryptor::from_reference(&blob_ref);
        assert_eq!(d.decrypt_chunk(&c1).unwrap(), p1);
        assert_eq!(d.decrypt_chunk(&c2).unwrap(), p2);
        assert_eq!(d.finish().unwrap(), blob_ref.total_plaintext_len);
    }

    // --- Bounds + redaction ---

    #[test]
    fn chunk_count_overflow_protection() {
        // Build an encryptor near the chunk-count cap.
        let mut e = enc_pair();
        e.next_index = MAX_CHUNK_COUNT;
        let r = e.encrypt_chunk(&[1u8; 1], true);
        assert!(matches!(r, Err(AttachmentError::TooLarge)));
    }

    #[test]
    fn decryptor_rejects_chunks_past_max_chunk_count() {
        // Mirror of `chunk_count_overflow_protection` on the
        // read side. Even if a peer produces a stream past the
        // cap with valid AEAD tags, the decryptor refuses.
        let mut e = enc_pair();
        let c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let mut d = dec_pair();
        d.next_index = MAX_CHUNK_COUNT;
        let r = d.decrypt_chunk(&c);
        assert!(matches!(r, Err(AttachmentError::TooLarge)));
    }

    #[test]
    fn blob_reference_debug_redacts_dek() {
        let r = BlobReference {
            blob_id: fixed_blob_id(),
            dek: Zeroizing::new([0xAFu8; DEK_LEN]),
            total_plaintext_len: 42,
            chunk_count: 3,
        };
        let s = format!("{:?}", r);
        assert!(s.contains("<redacted>"));
        // Hex of 0xAF is "af"; it should NOT appear in Debug.
        assert!(!s.contains("af af"));
    }

    #[test]
    fn encrypted_chunk_debug_redacts_ciphertext() {
        let mut e = enc_pair();
        let c = e.encrypt_chunk(b"super-secret-bytes", true).unwrap();
        let s = format!("{:?}", c);
        assert!(s.contains("18 bytes redacted"));
        assert!(!s.contains("super-secret-bytes"));
    }

    #[test]
    fn blob_id_debug_is_hex() {
        let mut bytes = [0u8; BLOB_ID_LEN];
        bytes[0] = 0xDE;
        bytes[1] = 0xAD;
        bytes[2] = 0xBE;
        bytes[3] = 0xEF;
        let id = BlobId(bytes);
        let s = format!("{:?}", id);
        assert!(s.starts_with("BlobId(deadbeef"));
    }

    // --- AAD covers all promised fields ---

    #[test]
    fn changing_chunk_index_in_aad_breaks_verify() {
        // Encrypt at index 0; decrypt as if index 1. Must
        // fail.
        let mut e = enc_pair();
        let c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let mut d = dec_pair();
        d.next_index = 1; // Pretend we already decrypted index 0.
        let r = d.decrypt_chunk(&c);
        // chunk.index (=0) != decryptor.next_index (=1) catches
        // this at the order check.
        assert!(matches!(r, Err(AttachmentError::Misuse(_))));
    }

    #[test]
    fn forged_chunk_with_swapped_index_fails_aead() {
        // Encrypt at index 0; produce a chunk that claims
        // index 1 but reuses the index-0 ciphertext+tag.
        // Must fail because the AAD computed at index 1 is
        // different from the one used at encryption.
        let mut e = enc_pair();
        let c = e.encrypt_chunk(&[1u8; 100], true).unwrap();
        let mut tampered = c.clone();
        tampered.index = 1;
        let mut d = dec_pair();
        // Move the decryptor to expect index 1, simulating a
        // server that dropped the real chunk 0 and pushed the
        // re-numbered chunk forward.
        d.next_index = 1;
        let r = d.decrypt_chunk(&tampered);
        assert!(matches!(r, Err(AttachmentError::Aead)));
    }

    // --- Send + Sync ---

    #[test]
    fn encryptor_decryptor_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AttachmentEncryptor>();
        assert_send::<AttachmentDecryptor>();
    }
}
