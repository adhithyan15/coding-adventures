//! VLT-PM47 chunked encrypted attachments.
//!
//! # Why a file is not a field
//!
//! Every other thing a vault-pm item holds is small enough to sit inside one
//! item revision. An attachment is not, and the bound that stops it is not the
//! one people expect. `MAX_PLAINTEXT_BYTES` — this crate's 16 MiB gate on what
//! the AEAD layer will seal in one frame — is the *outer* of two ceilings.
//! `canonical-cbor`'s `MAX_ENCODED_SIZE` is 1 MiB, it applies to every single
//! CBOR value this product emits, and it is the tighter one. VLT-PM05 §13.1
//! records what the gap between them cost: values in it are legal to hold,
//! legal to decode, and illegal to re-encode, and every encode in that range
//! used to abort the process.
//!
//! So an attachment is stored as fixed 64 KiB pieces. The number that matters
//! is that one sealed chunk encodes to about 65,600 bytes against a 1 MiB
//! ceiling — sixteen times the headroom, on a value whose size is a constant
//! of the format and cannot vary with the file.
//!
//! # The two layers of encryption, and what each one is for
//!
//! Each piece is sealed twice, and the layers authenticate different things.
//!
//! ```text
//!   plaintext[i*64KiB .. ]
//!        |
//!        |  VLT14 chunk AEAD  (XChaCha20-Poly1305, per-attachment DEK)
//!        |  nonce = blob_id || index_be || 0000
//!        |  aad   = "VAT1" || blob_id || index_be || is_final || 0
//!        v
//!   EncryptedChunk { blob_id, index, is_final, ciphertext, tag }
//!        |
//!        |  canonical CBOR, kind = AttachmentChunk
//!        |  vault-pm object envelope (fresh random DEK, wrapped under the
//!        |  vault object-wrap key; aad binds suite, vault, kind)
//!        v
//!   ObjectFrameV1  ->  ObjectId = SHA256("VPM-OBJECT-ID-v1" || frame)
//! ```
//!
//! The outer layer binds the bytes to *this vault* and *this kind*, and gives
//! them an address. It says nothing about which attachment a chunk belongs to
//! or where in it the chunk sits. The inner layer binds the bytes to *this
//! blob* at *this index* with *this* finality, which is what makes "chunk 4 of
//! attachment A substituted for chunk 4 of attachment B" a tag failure instead
//! of a silent wrong answer. Content addressing does not supply that: it
//! guarantees an object's bytes, not that a manifest lists the right objects in
//! the right order. The cost is sixteen bytes per 64 KiB — 0.02%.
//!
//! # Nonce discipline
//!
//! VLT14 derives chunk nonces from a counter, which is against this crate's
//! rule that every nonce is caller-injected independent randomness. The rule is
//! not broken, because the two live at different layers: the object frames keep
//! their random nonces, and the counter nonces are safe under VLT14's stated
//! condition — a `(blob_id, dek)` pair never reused across streams. Both the
//! 128-bit blob id and the 256-bit DEK come fresh out of the caller's entropy
//! block on every attach, so no two streams can collide even on the same file.
//! [`AttachmentEncryptor::new_random`], which would reach for the OS CSPRNG
//! directly and break this crate's determinism, is deliberately not used.

use crate::codec::{expected_chunk_count, validate_attachment_name};
use crate::ApplicationError;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_attachments::{
    AttachmentDecryptor, AttachmentEncryptor, BlobId, EncryptedChunk, CHUNK_SIZE, DEK_LEN,
};
use coding_adventures_vault_pm_domain::AttachmentId;
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};

/// Plaintext bytes carried by one attachment chunk.
///
/// VLT14's `CHUNK_SIZE`, restated here so this crate's bounds all read from
/// one place. It is not re-decided: passing anything else to VLT14's encryptor
/// is refused by the encryptor.
pub const ATTACHMENT_CHUNK_BYTES: usize = CHUNK_SIZE;

/// Length of one attachment's VLT14 data-encryption key.
pub const ATTACHMENT_DEK_BYTES: usize = DEK_LEN;

/// Largest plaintext this product will store as one attachment.
///
/// Deliberately *equal to* `MAX_PLAINTEXT_BYTES`, and the equality is the
/// argument: an attachment can never be larger than a plaintext this product
/// already accepts in a single sealed frame, so attachments do not become a
/// bigger door than records. A number chosen for user comfort would have had
/// no such property and would need defending separately every time the frame
/// bound moved.
pub const MAX_ATTACHMENT_BYTES: usize = crate::codec::MAX_PLAINTEXT_BYTES;

/// Largest chunk count one attachment may occupy.
///
/// Derived rather than written, because the whole subject of this module is
/// what happens when two bounds that should agree are stated twice.
pub const MAX_ATTACHMENT_CHUNKS: usize = MAX_ATTACHMENT_BYTES.div_ceil(ATTACHMENT_CHUNK_BYTES);

/// A generous upper bound on everything a chunk object adds to its payload.
///
/// The real figure is about sixty bytes — a 16-byte tag, a 16-byte blob id, an
/// index, a flag, a version, a kind, and CBOR framing. A kilobyte is used here
/// because the assertion below should keep holding across a format revision
/// that adds a field, and should stop holding if one ever adds a *lot*.
const MAX_CHUNK_OBJECT_FRAMING_BYTES: usize = 1024;

/// The relation this whole design turns on, checked by the compiler.
///
/// One sealed chunk object is at most `ATTACHMENT_CHUNK_BYTES` plus the
/// framing above, and requiring fifteen of those to fit under
/// `canonical-cbor`'s `MAX_ENCODED_SIZE` is what makes "an attachment chunk
/// cannot reach the codec ceiling" a fact rather than an expectation. Unlike a
/// test, this one cannot be skipped: a change to either constant stops the
/// crate compiling.
///
/// Fifteen rather than sixteen because sixteen chunks of payload alone come to
/// exactly the ceiling, leaving no room for the framing — the encoded object is
/// what has to fit, not the plaintext it carries.
const _: () = assert!(
    (ATTACHMENT_CHUNK_BYTES + MAX_CHUNK_OBJECT_FRAMING_BYTES) * 15
        <= coding_adventures_canonical_cbor::MAX_ENCODED_SIZE,
    "one encoded attachment chunk must sit far under canonical-CBOR's ceiling"
);

/// One attachment must never be able to overflow one publication.
const _: () = assert!(
    MAX_ATTACHMENT_CHUNKS + 4 <= coding_adventures_vault_pm_repository::MAX_PUBLICATION_OBJECTS,
    "chunks plus manifest, revision, catalog and audit event must fit one commit"
);

/// Longest attachment base name, in bytes.
///
/// The widest name every filesystem this product targets accepts, so a name
/// that survives ingest is a name that could have been a file. That is a
/// property of the *source*; VLT-PM47 §4.6 is emphatic that it is not a promise
/// about any destination, because no code path turns a stored name into a path.
pub const MAX_ATTACHMENT_NAME_BYTES: usize = 255;

/// Non-secret projection of one stored attachment.
///
/// The content hash is rendered on purpose: it is what makes the
/// byte-identical round trip checkable by a person, and it is a hash of
/// plaintext they can already obtain, so it discloses nothing new.
///
/// # Why this one does not wipe its name, and the other two do
///
/// The attachment name is **metadata**, of the same class as an item title:
/// `attachment list` prints it on ordinary standard output, so treating it as
/// a secret here would be a claim the very next line of the product breaks.
/// This type holds nothing else, so it holds nothing to wipe, and it is
/// `Clone` because a caller may reasonably keep one.
///
/// [`AttachmentContentV1`] and `AttachmentManifestV1` do wipe theirs, and that
/// is not a contradiction: both also hold a real secret — the plaintext and
/// the key — and both wipe *everything they hold* rather than maintaining a
/// per-field classification that a later field could be added on the wrong
/// side of. Wiping a non-secret costs a memset; getting the classification
/// wrong once costs a key.
#[derive(Clone, PartialEq, Eq)]
pub struct AttachmentSummaryV1 {
    attachment_id: AttachmentId,
    name: String,
    total_plaintext_len: u64,
    content_sha256: [u8; 32],
}

impl AttachmentSummaryV1 {
    pub(crate) const fn new(
        attachment_id: AttachmentId,
        name: String,
        total_plaintext_len: u64,
        content_sha256: [u8; 32],
    ) -> Self {
        Self {
            attachment_id,
            name,
            total_plaintext_len,
            content_sha256,
        }
    }

    /// Return the attachment identity.
    pub const fn attachment_id(&self) -> AttachmentId {
        self.attachment_id
    }

    /// Borrow the stored base name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the plaintext byte length.
    pub const fn total_plaintext_len(&self) -> u64 {
        self.total_plaintext_len
    }

    /// Borrow the SHA-256 of the complete plaintext.
    pub const fn content_sha256(&self) -> &[u8; 32] {
        &self.content_sha256
    }
}

impl Debug for AttachmentSummaryV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AttachmentSummaryV1")
            .field("attachment_id", &"<redacted>")
            .field("name", &"<redacted>")
            .field("total_plaintext_len", &self.total_plaintext_len)
            .finish()
    }
}

/// One reassembled attachment, released only after its audit event is durable.
///
/// No `Debug`, no `Display`, no `Clone`, wiped on drop — the same treatment
/// `RevealedSecretV1` and `TotpCodeV1` get, for the same reason.
pub struct AttachmentContentV1 {
    name: String,
    bytes: Zeroizing<Vec<u8>>,
}

impl AttachmentContentV1 {
    pub(crate) const fn new(name: String, bytes: Zeroizing<Vec<u8>>) -> Self {
        Self { name, bytes }
    }

    /// Borrow the stored base name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Explicitly borrow the plaintext.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Zeroize for AttachmentContentV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.name.zeroize();
    }
}

impl Drop for AttachmentContentV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Everything the chunking pass produced, ready to be sealed and published.
pub(crate) struct ChunkedAttachmentV1 {
    pub(crate) chunks: Vec<EncryptedChunk>,
    pub(crate) total_plaintext_len: u64,
    pub(crate) content_sha256: [u8; 32],
}

/// Split one plaintext into VLT14 sealed chunks.
///
/// `blob_id` and `dek` come from the caller's reserved entropy block, never
/// from an ambient CSPRNG — see this module's header on nonce discipline.
///
/// The bound is applied to the input before a single chunk is sealed, so an
/// oversized attachment costs no entropy, no allocation past the input itself,
/// and no partial state.
pub(crate) fn chunk_attachment(
    plaintext: &[u8],
    blob_id: BlobId,
    dek: Zeroizing<[u8; ATTACHMENT_DEK_BYTES]>,
) -> Result<ChunkedAttachmentV1, ApplicationError> {
    if plaintext.is_empty() {
        return Err(ApplicationError::InvalidInput);
    }
    if plaintext.len() > MAX_ATTACHMENT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    let content_sha256 = sha256(plaintext);
    let mut encryptor = AttachmentEncryptor::from_dek(blob_id, dek);
    let mut chunks = Vec::with_capacity(expected_chunk_count(plaintext.len() as u64));
    let mut offset = 0usize;
    while offset < plaintext.len() {
        let end = (offset + ATTACHMENT_CHUNK_BYTES).min(plaintext.len());
        let is_final = end == plaintext.len();
        let chunk = encryptor
            .encrypt_chunk(&plaintext[offset..end], is_final)
            .map_err(map_attachment_error)?;
        chunks.push(chunk);
        offset = end;
    }
    let reference = encryptor
        .finalize_reference()
        .map_err(map_attachment_error)?;
    // The encryptor's own accounting must agree with ours. It is the same
    // arithmetic twice, which is exactly the situation where a disagreement
    // means one of the two is wrong rather than that the input was bad.
    if reference.total_plaintext_len != plaintext.len() as u64
        || reference.chunk_count as usize != chunks.len()
    {
        return Err(ApplicationError::InternalInvariant);
    }
    Ok(ChunkedAttachmentV1 {
        chunks,
        total_plaintext_len: plaintext.len() as u64,
        content_sha256,
    })
}

/// Reassemble one attachment from its chunks.
///
/// `fetch` yields the chunk stored at each manifest reference, in index order.
/// Every failure mode below is reachable from a synchronising peer, and none of
/// them is a panic:
///
/// | Malformation | Caught by |
/// |---|---|
/// | declared length above the ceiling | the bound below, before allocation |
/// | chunk out of order, or from another blob | VLT14's decryptor |
/// | a chunk promoted to final, or one after final | VLT14's AAD binding |
/// | any byte flipped | the chunk AEAD tag |
/// | the stream truncated | `AttachmentDecryptor::finish` |
/// | a length that disagrees with the manifest | the equality below |
/// | content that disagrees with the manifest hash | the comparison below |
///
/// The last two are not redundant with the signed commit chain above the
/// manifest. VLT14 v1 commits `0` rather than the real total in every chunk's
/// associated data — it cannot know the total until the stream ends — and its
/// documentation names verifying `finish()` against the stored total as the
/// caller's duty. Re-deriving both is what makes a byte-identical round trip a
/// checked property rather than an assumption.
pub(crate) fn reassemble_attachment(
    blob_id: BlobId,
    dek: Zeroizing<[u8; ATTACHMENT_DEK_BYTES]>,
    total_plaintext_len: u64,
    content_sha256: &[u8; 32],
    chunk_count: usize,
    mut fetch: impl FnMut(usize) -> Result<EncryptedChunk, ApplicationError>,
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    if total_plaintext_len > MAX_ATTACHMENT_BYTES as u64 || chunk_count > MAX_ATTACHMENT_CHUNKS {
        return Err(ApplicationError::BoundExceeded);
    }
    if chunk_count != expected_chunk_count(total_plaintext_len) {
        return Err(ApplicationError::IntegrityFailure);
    }
    // Only now, past both bounds, is a declared length allowed to size a
    // buffer. Reversing these two statements is the bug this table exists to
    // prevent.
    let mut plaintext = Zeroizing::new(Vec::with_capacity(total_plaintext_len as usize));
    let mut decryptor = AttachmentDecryptor::new(blob_id, dek);
    for index in 0..chunk_count {
        let chunk = fetch(index)?;
        let mut piece = Zeroizing::new(
            decryptor
                .decrypt_chunk(&chunk)
                .map_err(map_attachment_error)?,
        );
        plaintext.extend_from_slice(&piece);
        piece.zeroize();
    }
    let observed = decryptor.finish().map_err(map_attachment_error)?;
    if observed != total_plaintext_len || plaintext.len() as u64 != total_plaintext_len {
        return Err(ApplicationError::IntegrityFailure);
    }
    if &sha256(&plaintext) != content_sha256 {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(plaintext)
}

/// Extract and validate the base name of a source path.
///
/// The directory is never stored: VLT-PM15 §5 forbids a source path in an
/// audit event, and there is no reason for the vault to hold one either.
pub fn attachment_name_from_path(path: &str) -> Result<String, ApplicationError> {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .ok_or(ApplicationError::InvalidInput)?;
    validate_attachment_name(name)?;
    Ok(name.to_string())
}

/// Map VLT14's closed taxonomy onto this crate's.
///
/// `TooLarge` is a bound, everything else is a malformed or misused stream.
/// The distinction matters for the same reason `map_encode_error` keeps size
/// and integrity apart: an operator told "too large" is told to use a smaller
/// file, and an operator told "integrity failure" is told their store is
/// damaged. Saying the wrong one is worse than saying nothing.
fn map_attachment_error(
    error: coding_adventures_vault_attachments::AttachmentError,
) -> ApplicationError {
    use coding_adventures_vault_attachments::AttachmentError;
    match error {
        AttachmentError::TooLarge => ApplicationError::BoundExceeded,
        AttachmentError::InvalidParameter(_) | AttachmentError::Csprng(_) => {
            ApplicationError::InvalidInput
        }
        AttachmentError::Misuse(_)
        | AttachmentError::Aead
        | AttachmentError::Truncated
        | AttachmentError::AfterFinal => ApplicationError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Result::unwrap_err` needs the success type to implement `Debug`, and
    /// deliberately none of the values this module produces does — a plaintext
    /// a stray `dbg!` could print is the thing the redaction discipline exists
    /// to prevent. So the tests assert on errors through this rather than by
    /// weakening the types.
    fn expect_err<T>(result: Result<T, ApplicationError>) -> ApplicationError {
        match result {
            Ok(_) => panic!("expected a closed error"),
            Err(error) => error,
        }
    }

    fn dek() -> Zeroizing<[u8; ATTACHMENT_DEK_BYTES]> {
        Zeroizing::new([0x5a; ATTACHMENT_DEK_BYTES])
    }

    fn blob() -> BlobId {
        BlobId([0x11; 16])
    }

    fn round_trip(plaintext: &[u8]) -> Zeroizing<Vec<u8>> {
        let chunked = chunk_attachment(plaintext, blob(), dek()).unwrap();
        let chunks = chunked.chunks.clone();
        reassemble_attachment(
            blob(),
            dek(),
            chunked.total_plaintext_len,
            &chunked.content_sha256,
            chunks.len(),
            |index| Ok(chunks[index].clone()),
        )
        .unwrap()
    }

    /// The ceilings must be *derived*. A second literal is a second thing that
    /// can drift, and drifting bounds are the subject of this whole module.
    #[test]
    fn the_bounds_agree_with_the_ceilings_they_are_derived_from() {
        assert_eq!(ATTACHMENT_CHUNK_BYTES, 64 * 1024);
        assert_eq!(MAX_ATTACHMENT_BYTES, crate::codec::MAX_PLAINTEXT_BYTES);
        assert_eq!(MAX_ATTACHMENT_CHUNKS, 256);
        assert_eq!(
            MAX_ATTACHMENT_CHUNKS * ATTACHMENT_CHUNK_BYTES,
            MAX_ATTACHMENT_BYTES
        );
    }

    #[test]
    fn multi_chunk_plaintexts_round_trip_byte_identically() {
        for length in [
            1usize,
            ATTACHMENT_CHUNK_BYTES - 1,
            ATTACHMENT_CHUNK_BYTES,
            ATTACHMENT_CHUNK_BYTES + 1,
            3 * ATTACHMENT_CHUNK_BYTES,
            3 * ATTACHMENT_CHUNK_BYTES + 7,
        ] {
            let plaintext: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            let restored = round_trip(&plaintext);
            assert_eq!(restored.as_slice(), plaintext.as_slice(), "length {length}");
            let chunked = chunk_attachment(&plaintext, blob(), dek()).unwrap();
            assert_eq!(
                chunked.chunks.len(),
                length.div_ceil(ATTACHMENT_CHUNK_BYTES).max(1)
            );
            assert!(chunked.chunks.last().unwrap().is_final);
            assert!(!chunked.chunks[..chunked.chunks.len() - 1]
                .iter()
                .any(|chunk| chunk.is_final));
        }
    }

    #[test]
    fn empty_and_oversized_inputs_are_refused_without_a_panic() {
        assert_eq!(
            expect_err(chunk_attachment(&[], blob(), dek())),
            ApplicationError::InvalidInput
        );
        let oversized = vec![0u8; MAX_ATTACHMENT_BYTES + 1];
        assert_eq!(
            expect_err(chunk_attachment(&oversized, blob(), dek())),
            ApplicationError::BoundExceeded
        );
        // Exactly at the ceiling still works, so the boundary is exact rather
        // than approximate.
        let exact = vec![7u8; MAX_ATTACHMENT_BYTES];
        let chunked = chunk_attachment(&exact, blob(), dek()).unwrap();
        assert_eq!(chunked.chunks.len(), MAX_ATTACHMENT_CHUNKS);
    }

    /// Every one of these is authorable by a synchronising peer, and the point
    /// is as much that none of them aborts as that each is caught.
    #[test]
    fn peer_authored_malformations_are_closed_errors() {
        let plaintext = vec![3u8; 3 * ATTACHMENT_CHUNK_BYTES + 11];
        let chunked = chunk_attachment(&plaintext, blob(), dek()).unwrap();
        let good = chunked.chunks.clone();
        let total = chunked.total_plaintext_len;
        let hash = chunked.content_sha256;
        let count = good.len();

        // A declared length far above the ceiling must be refused before it is
        // ever used as a capacity.
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                u64::MAX,
                &hash,
                count,
                |index| Ok(good[index].clone())
            )),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &hash,
                MAX_ATTACHMENT_CHUNKS + 1,
                |index| Ok(good[index].clone())
            )),
            ApplicationError::BoundExceeded
        );
        // A chunk count that does not fit the declared length.
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &hash,
                count - 1,
                |index| Ok(good[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );

        // Reordered chunks.
        let mut swapped = good.clone();
        swapped.swap(0, 1);
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &hash,
                count,
                |index| Ok(swapped[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );

        // A flipped ciphertext byte.
        let mut tampered = good.clone();
        tampered[1].ciphertext[0] ^= 0x01;
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &hash,
                count,
                |index| Ok(tampered[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );

        // A chunk from another blob at the same index.
        let other = chunk_attachment(&plaintext, BlobId([0x22; 16]), dek()).unwrap();
        let mut crossed = good.clone();
        crossed[1] = other.chunks[1].clone();
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &hash,
                count,
                |index| Ok(crossed[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );

        // A content hash that disagrees with the bytes.
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                total,
                &[0u8; 32],
                count,
                |index| Ok(good[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );

        // The honest stream still succeeds, so none of the above passed by
        // breaking the fixture.
        assert!(
            reassemble_attachment(blob(), dek(), total, &hash, count, |index| Ok(
                good[index].clone()
            ))
            .is_ok()
        );
    }

    /// A stream that ends without a final chunk is the case VLT14's `finish`
    /// exists for: a store that drops the tail is otherwise indistinguishable
    /// from a reader who has not got there yet.
    #[test]
    fn a_stream_with_no_final_chunk_is_truncated() {
        let plaintext = vec![9u8; 2 * ATTACHMENT_CHUNK_BYTES];
        let chunked = chunk_attachment(&plaintext, blob(), dek()).unwrap();
        let mut chunks = chunked.chunks.clone();
        // Re-seal the tail as the non-final middle chunk of a longer stream,
        // which is what a truncating store would present.
        let longer = vec![9u8; 3 * ATTACHMENT_CHUNK_BYTES];
        let extended = chunk_attachment(&longer, blob(), dek()).unwrap();
        chunks[1] = extended.chunks[1].clone();
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                chunked.total_plaintext_len,
                &chunked.content_sha256,
                2,
                |index| Ok(chunks[index].clone())
            )),
            ApplicationError::IntegrityFailure
        );
    }

    /// A missing chunk object surfaces the fetcher's own error rather than
    /// being rewritten into something that sounds like tampering.
    #[test]
    fn a_missing_chunk_object_reports_not_found() {
        let plaintext = vec![1u8; 2 * ATTACHMENT_CHUNK_BYTES];
        let chunked = chunk_attachment(&plaintext, blob(), dek()).unwrap();
        assert_eq!(
            expect_err(reassemble_attachment(
                blob(),
                dek(),
                chunked.total_plaintext_len,
                &chunked.content_sha256,
                chunked.chunks.len(),
                |index| if index == 1 {
                    Err(ApplicationError::NotFound)
                } else {
                    Ok(chunked.chunks[index].clone())
                }
            )),
            ApplicationError::NotFound
        );
    }

    #[test]
    fn names_are_taken_from_the_base_and_validated_rather_than_repaired() {
        assert_eq!(
            attachment_name_from_path("/home/ada/recovery codes.pdf").unwrap(),
            "recovery codes.pdf"
        );
        assert_eq!(
            attachment_name_from_path(r"C:\Users\ada\key.pem").unwrap(),
            "key.pem"
        );
        // Taking the base name is what makes a traversal attempt inert: the
        // directory components are simply not part of what is stored, so
        // there is nothing left to traverse with.
        assert_eq!(
            attachment_name_from_path("../../etc/passwd").unwrap(),
            "passwd"
        );
        for rejected in [
            "",
            "/",
            ".",
            "..",
            "a\0b",
            "a\nb",
            "/tmp/",
            "a/../b/",
            // Category Cf and Zl/Zp. `char::is_control` lets every one of
            // these through, and each renders as a name other than the one
            // stored — which matters because the operator picks an attachment
            // to export by reading that rendering.
            "recovery\u{202E}gnp.exe",
            "invoice\u{200F}fdp.exe",
            "quiet\u{200B}name.pdf",
            "\u{FEFF}leading-mark.pdf",
            "two\u{2028}rows.pdf",
            "two\u{2029}rows.pdf",
            "isolate\u{2066}name.pdf",
            "soft\u{00AD}hyphen.pdf",
            "mongolian\u{180E}separator.pdf",
            "tagged\u{E0041}name.pdf",
            "arabic\u{0600}number-sign.pdf",
        ] {
            assert_eq!(
                expect_err(attachment_name_from_path(rejected)),
                ApplicationError::InvalidInput,
                "{rejected:?} must be refused"
            );
        }
        // Ordinary non-ASCII names are still perfectly acceptable: the
        // rejection is of characters that change what a name *looks like*,
        // not of anyone's alphabet.
        for accepted in [
            "résumé.pdf",
            "recuperación.txt",
            "восстановление.bin",
            "回復コード.pdf",
        ] {
            assert!(
                validate_attachment_name(accepted).is_ok(),
                "{accepted:?} must be accepted"
            );
        }
        assert!(validate_attachment_name(&"a".repeat(MAX_ATTACHMENT_NAME_BYTES)).is_ok());
        assert_eq!(
            expect_err(validate_attachment_name(
                &"a".repeat(MAX_ATTACHMENT_NAME_BYTES + 1)
            )),
            ApplicationError::InvalidInput
        );
    }

    #[test]
    fn projections_redact_and_wipe() {
        let summary = AttachmentSummaryV1::new(
            AttachmentId::new([1; 16]),
            "secret-name.pdf".to_string(),
            10,
            [2; 32],
        );
        let rendered = format!("{summary:?}");
        assert!(!rendered.contains("secret-name"));
        assert!(rendered.contains("total_plaintext_len: 10"));
        assert_eq!(summary.name(), "secret-name.pdf");
        assert_eq!(summary.attachment_id(), AttachmentId::new([1; 16]));
        assert_eq!(summary.content_sha256(), &[2; 32]);
        assert_eq!(summary.total_plaintext_len(), 10);
        assert_eq!(summary.clone(), summary);

        let content =
            AttachmentContentV1::new("name.bin".to_string(), Zeroizing::new(vec![4, 5, 6]));
        assert_eq!(content.name(), "name.bin");
        assert_eq!(content.as_bytes(), &[4, 5, 6]);
    }

    /// VLT14's error taxonomy has to reach this crate's without collapsing a
    /// size problem into an integrity problem, for `map_encode_error`'s
    /// reason: the two tell an operator to do different things.
    #[test]
    fn vault14_errors_keep_size_and_integrity_apart() {
        use coding_adventures_vault_attachments::AttachmentError;
        assert_eq!(
            map_attachment_error(AttachmentError::TooLarge),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_attachment_error(AttachmentError::InvalidParameter("x")),
            ApplicationError::InvalidInput
        );
        assert_eq!(
            map_attachment_error(AttachmentError::Csprng("x".to_string())),
            ApplicationError::InvalidInput
        );
        for integrity in [
            AttachmentError::Misuse("x"),
            AttachmentError::Aead,
            AttachmentError::Truncated,
            AttachmentError::AfterFinal,
        ] {
            assert_eq!(
                map_attachment_error(integrity),
                ApplicationError::IntegrityFailure
            );
        }
    }
}
