use crate::{
    ApplicationError, ObjectKind, ATTACHMENT_CHUNK_BYTES, ATTACHMENT_DEK_BYTES,
    MAX_ATTACHMENT_BYTES, MAX_ATTACHMENT_CHUNKS, MAX_ATTACHMENT_NAME_BYTES,
};
use coding_adventures_canonical_cbor::{
    decode, encode, try_encode, CborError, CborValue, MAX_ENCODED_SIZE,
};
use coding_adventures_vault_attachments::{BlobId, EncryptedChunk};
use coding_adventures_vault_pm_audit::SignedAuditEventV1;
use coding_adventures_vault_pm_domain::{
    AttachmentId, AttachmentManifestId, CollectionId, ContentType, ItemCandidate, ItemDocument,
    ItemId, ItemState, LwwRegister, ObservedSet, OperationId, RevisionId, Tombstone,
    MAX_OBSERVED_VALUES,
};
use coding_adventures_vault_pm_format::{
    CommitV1, DeviceCertificateV1, DeviceId, ObjectId, VaultId,
};
use coding_adventures_vault_records::{
    decode_record, encode_opaque, encode_record, AnyRecord, VaultRecordError,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const VERSION: u64 = 1;
const LIVE_STATE: u64 = 1;
const TOMBSTONE_STATE: u64 = 2;
pub(crate) const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_CANDIDATES_PER_ITEM: usize = 16;

// ─────────────────────────────────────────────────────────────────────
// Catalog entry bounds — derived from MAX_ENCODED_SIZE, not eyeballed
// ─────────────────────────────────────────────────────────────────────
//
// This used to be one flat, round `100_000` used for admission, decode,
// and cross-head merging alike. `100_000` was never reachable: canonical-
// CBOR's own 1 MiB `MAX_ENCODED_SIZE` (this crate's *inner* bound, see
// `CatalogV1::encode`'s doc comment and VLT-PM05 §13.1) refuses to emit a
// catalog anywhere near that many entries. An ordinary vault crossed the
// real ceiling somewhere below twenty thousand items, and because catalog
// entries are never removed — a deleted item becomes a tombstone *entry*,
// not a smaller one — nothing this product does from inside a session
// could ever bring an over-ceiling catalog back under it. Every mutation
// that had to re-encode the catalog, delete included, failed the same way
// forever. See VLT-PM05 §13.2 for the full account.
//
// The fix is two numbers, not one, because "can we still open a catalog
// bigger than what we would create" and "how big a catalog are we willing
// to keep growing" are different questions with different backward-
// compatibility answers:

/// Exact byte cost of one catalog entry with exactly one candidate
/// revision: `{1: bytes(item_id), 2: [bytes(revision_id)]}`.
///
/// `ItemId` is a fixed 16-byte value and `RevisionId` a fixed 32-byte
/// value, and canonical-CBOR's header for a definite-length byte string
/// is fully determined by its length, never its content — so this is not
/// an estimate, it is an exact count:
///
/// ```text
///   entry map header (2 pairs, < 24)              1
///   key 1 (unsigned < 24)                          1
///   item id   bytes header (len 16, < 24)          1
///             item id bytes                       16
///   key 2 (unsigned < 24)                          1
///   candidates  array header (1 elem, < 24)        1
///     revision id  bytes header (len 32, >= 24)    2
///                  revision id bytes               32
///                                                ----
///                                                  55
/// ```
///
/// One candidate is the cheapest an entry can be — a second candidate
/// (a conflict) adds 34 more bytes, never fewer — so this is a lower
/// bound on real per-entry cost and therefore an *upper* bound on how
/// many entries any valid catalog can hold. Pinned against the real
/// encoder by `catalog_entry_byte_cost_is_exact` below.
const CATALOG_ENTRY_BYTES: usize = 55;

/// Conservative upper bound on the bytes *outside* the per-entry cost:
/// the outer map's three fields (version, kind, entries array) plus the
/// entries array's own length header, which grows from 1 byte
/// (< 24 elements) to 2 (< 256), 3 (< 65,536), or 5 (< 2^32) as the
/// catalog grows. 16 bytes covers all of that with room to spare in the
/// size range this bound actually operates in (tens of thousands of
/// entries, a 3-byte array header) without needing a separate case for
/// the exact crossover point.
const CATALOG_FRAME_OVERHEAD_BYTES: usize = 16;

/// The real ceiling: the largest entry count *any* valid encode of a
/// `CatalogV1` can reach, on this device or any other honouring the same
/// wire format. Because `CATALOG_ENTRY_BYTES` is the cheapest an entry
/// can ever be, no catalog — however its entries are distributed between
/// single-candidate and conflicted — can exceed this many entries and
/// still fit inside `MAX_ENCODED_SIZE`. That makes it safe to use as a
/// decode-time bound with zero backward-compatibility cost: nothing
/// honestly produced by this wire format, past or present, can be turned
/// away by it. Only a peer that does not honour `MAX_ENCODED_SIZE` at all
/// — hand-crafting bytes rather than running this encoder — can ever
/// author a catalog object this large.
pub(crate) const MAX_ENCODABLE_CATALOG_ENTRIES: usize =
    (MAX_ENCODED_SIZE - CATALOG_FRAME_OVERHEAD_BYTES) / CATALOG_ENTRY_BYTES;

/// Entries of headroom subtracted from [`MAX_ENCODABLE_CATALOG_ENTRIES`]
/// to get the admission-time ceiling below. Two things eat into the exact
/// ceiling without changing entry *count*: items that have gone through
/// conflict resolution carry more than one candidate revision (34 extra
/// bytes each), and this margin needs to absorb that without a per-entry
/// accounting. 1,000 entries of margin is 55,000 bytes of slack — room
/// for on the order of a thousand simultaneously-conflicted items, far
/// past what concurrent editing of a personal vault produces in
/// practice. A vault that legitimately exceeds even that is the
/// pre-existing, tracked residual VLT-PM05 §13.2 names for the
/// conflicted-item case, not a new one this change introduces.
const CATALOG_ADMISSION_SAFETY_MARGIN: usize = 1_000;

/// The admission-time ceiling this device applies to catalogs it
/// constructs itself: [`MAX_ENCODABLE_CATALOG_ENTRIES`] minus
/// [`CATALOG_ADMISSION_SAFETY_MARGIN`]. Used by `validate_catalog`, and
/// therefore by every local mutation that builds a new `CatalogV1` —
/// `item add`, `item delete`, `item edit`, the authored conflict merges,
/// and portable import. Reaching this ceiling refuses the *next* item
/// admitted, with `BoundExceeded`, before the catalog is ever built —
/// which is the actual fix: the old code let admission run all the way
/// to a fictional `100_000` and only discovered the real ceiling on
/// re-encode, by which point every entry already admitted was
/// permanently stuck in an unencodable catalog. A catalog that stays
/// under this ceiling can always be edited, deleted from, and
/// re-encoded — the escape hatch VLT-PM05 §13.2 already relies on for
/// oversized individual records now also holds for the catalog as a
/// whole, because admission never lets the catalog reach a size that
/// escape hatch can't recover from.
pub(crate) const MAX_CATALOG_ENTRIES: usize =
    MAX_ENCODABLE_CATALOG_ENTRIES - CATALOG_ADMISSION_SAFETY_MARGIN;

/// Owner-private authority and device seeds persisted only as encrypted state.
#[derive(PartialEq, Eq)]
pub struct LocalSecretV1 {
    vault_id: VaultId,
    device_id: DeviceId,
    authority_seed: [u8; 32],
    device_signing_seed: [u8; 32],
    device_x25519_secret: [u8; 32],
}

impl LocalSecretV1 {
    /// Construct one complete V1 local secret record.
    pub const fn new(
        vault_id: VaultId,
        device_id: DeviceId,
        authority_seed: [u8; 32],
        device_signing_seed: [u8; 32],
        device_x25519_secret: [u8; 32],
    ) -> Self {
        Self {
            vault_id,
            device_id,
            authority_seed,
            device_signing_seed,
            device_x25519_secret,
        }
    }

    /// Return the vault binding.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Return the local certified device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Borrow the Ed25519 authority seed for an explicit unlocked operation.
    pub const fn authority_seed(&self) -> &[u8; 32] {
        &self.authority_seed
    }

    /// Borrow the local device Ed25519 seed for an explicit unlocked operation.
    pub const fn device_signing_seed(&self) -> &[u8; 32] {
        &self.device_signing_seed
    }

    /// Borrow the local device X25519 secret for an explicit unlocked operation.
    pub const fn device_x25519_secret(&self) -> &[u8; 32] {
        &self.device_x25519_secret
    }

    /// Encode the exact closed canonical V1 record.
    ///
    /// This is the one encode in this module that stays infallible, and
    /// provably so: every field is a fixed-width array (two 16-byte ids
    /// and three 32-byte seeds) plus a small integer, so the output is
    /// the same ~150 bytes on every call and cannot approach
    /// canonical-CBOR's 1 MiB ceiling or its depth cap. Nothing here is
    /// caller-sized, so there is no oversized input to report.
    pub fn encode(&self) -> Vec<u8> {
        encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, bytes(self.vault_id.as_bytes())),
            field(3, bytes(self.device_id.as_bytes())),
            field(4, bytes(&self.authority_seed)),
            field(5, bytes(&self.device_signing_seed)),
            field(6, bytes(&self.device_x25519_secret)),
        ]))
    }

    /// Strictly decode one exact closed canonical V1 record.
    pub fn decode(encoded: &[u8]) -> Result<Self, ApplicationError> {
        let mut fields = closed_fields(encoded, &[1, 2, 3, 4, 5, 6])?;
        check_version(take_uint(&mut fields, 1)?)?;
        Ok(Self {
            vault_id: VaultId::new(take_fixed(&mut fields, 2)?),
            device_id: DeviceId::new(take_fixed(&mut fields, 3)?),
            authority_seed: take_fixed(&mut fields, 4)?,
            device_signing_seed: take_fixed(&mut fields, 5)?,
            device_x25519_secret: take_fixed(&mut fields, 6)?,
        })
    }
}

impl Zeroize for LocalSecretV1 {
    fn zeroize(&mut self) {
        self.authority_seed.zeroize();
        self.device_signing_seed.zeroize();
        self.device_x25519_secret.zeroize();
    }
}

impl Drop for LocalSecretV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for LocalSecretV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalSecretV1(<redacted>)")
    }
}

/// Bounded immutable map from item IDs to current revision candidates.
#[derive(Clone, PartialEq, Eq)]
pub struct CatalogV1 {
    entries: BTreeMap<ItemId, Vec<RevisionId>>,
}

impl CatalogV1 {
    /// Validate and construct a canonical catalog snapshot from scratch.
    ///
    /// Every entry here is new growth — this is what portable import and
    /// tests use, where there is no "previous" catalog to compare against —
    /// so the whole entry count is checked against the tight, margined
    /// [`MAX_CATALOG_ENTRIES`]. An ordinary item mutation, which carries
    /// forward every entry that already existed and touches at most one,
    /// should use [`CatalogV1::new_for_mutation`] instead: that constructor
    /// is what keeps a catalog already sitting above the admission ceiling
    /// editable and deletable-from rather than merely openable. See that
    /// constructor's doc comment, and VLT-PM05 §13.4, for why the two must
    /// differ.
    pub fn new(entries: BTreeMap<ItemId, Vec<RevisionId>>) -> Result<Self, ApplicationError> {
        validate_catalog(&entries)?;
        Ok(Self { entries })
    }

    /// Validate and construct a catalog for one ordinary item mutation,
    /// given the entry count immediately before it.
    ///
    /// # Why this is not just `CatalogV1::new`
    ///
    /// `validate_catalog` (used by [`CatalogV1::new`]) rejects any entry
    /// count over the tight, margined [`MAX_CATALOG_ENTRIES`] — correct
    /// for admitting *new growth*, wrong for a mutation that does not grow
    /// the catalog at all. A single item mutation carries forward every
    /// entry `current_items` already had and touches exactly one: entry
    /// count after the mutation is `previous_entry_count` unchanged (edit,
    /// delete, restore, or conflict-resolution of an item that already had
    /// a catalog entry) or `previous_entry_count + 1` (a brand-new item,
    /// the only case that is genuinely new growth).
    ///
    /// Applying the tight bound unconditionally — as an earlier version of
    /// this fix did — reopened the exact bug VLT-PM05 §13.4 exists to
    /// close, just at a narrower band: a catalog synced from a peer, or
    /// grown under this device's own pre-fix admission policy, with an
    /// entry count anywhere in `(MAX_CATALOG_ENTRIES,
    /// MAX_ENCODABLE_CATALOG_ENTRIES]` would decode and open (§13.4's
    /// decode-time bound is the looser, proven ceiling) but then fail
    /// *every* subsequent mutation — including delete, the escape hatch —
    /// because rebuilding that same, unchanged entry count through
    /// `validate_catalog` always exceeded the tighter admission ceiling,
    /// regardless of whether the mutation added anything.
    ///
    /// So the ceiling this function applies depends on whether the
    /// mutation is growing the catalog: [`MAX_CATALOG_ENTRIES`] only if
    /// `entries.len() > previous_entry_count`, otherwise the looser
    /// [`MAX_ENCODABLE_CATALOG_ENTRIES`] — the same proven ceiling decode
    /// already trusts, so a catalog decode accepted can always still be
    /// edited, deleted from, and restored, not merely opened.
    pub(crate) fn new_for_mutation(
        entries: BTreeMap<ItemId, Vec<RevisionId>>,
        previous_entry_count: usize,
    ) -> Result<Self, ApplicationError> {
        let ceiling = if entries.len() > previous_entry_count {
            MAX_CATALOG_ENTRIES
        } else {
            MAX_ENCODABLE_CATALOG_ENTRIES
        };
        if entries.len() > ceiling {
            return Err(ApplicationError::BoundExceeded);
        }
        validate_catalog_structure(&entries)?;
        Ok(Self { entries })
    }

    /// Construct an empty generation-zero catalog.
    pub fn empty() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Borrow entries in deterministic item-ID order.
    pub const fn entries(&self) -> &BTreeMap<ItemId, Vec<RevisionId>> {
        &self.entries
    }

    /// Encode the exact closed canonical V1 application object.
    ///
    /// # Why this does not re-run `validate_catalog`
    ///
    /// An earlier version of this method called `validate_catalog(&self.
    /// entries)` here, applying the tight, margined `MAX_CATALOG_ENTRIES`
    /// unconditionally before encoding. That was wrong for exactly the
    /// catalogs [`CatalogV1::new_for_mutation`] exists to allow: a catalog
    /// already sitting above the admission ceiling — synced from a peer,
    /// or grown under this device's own pre-fix admission policy — whose
    /// entry count does not increase (an edit, a delete, a restore) is
    /// legitimately constructed by `new_for_mutation` against the looser,
    /// proven [`MAX_ENCODABLE_CATALOG_ENTRIES`], and re-checking it here
    /// against the tighter bound would refuse to encode a catalog that
    /// constructor had already correctly accepted — reopening, at the
    /// encode step, the exact bug VLT-PM05 §13.4 exists to close. See that
    /// constructor's doc comment for the full account.
    ///
    /// `self.entries` was already validated by whichever constructor built
    /// this value ([`CatalogV1::new`], [`CatalogV1::new_for_mutation`], or
    /// [`CatalogV1::empty`]), each against the ceiling appropriate to how
    /// it was built. This encode's own job is only to confirm the *bytes*
    /// still fit `MAX_ENCODED_SIZE` — the inner bound this crate's own
    /// 16 MiB plaintext gate does not itself enforce (VLT-PM05 §13.1) —
    /// which `encode_catalog_entries` checks unconditionally regardless of
    /// entry count. That inner check is what actually can still fail here:
    /// a conflicted item carries more than one candidate revision, and
    /// enough conflicted entries at once can exhaust a margin no entry-
    /// count ceiling accounts for. It reports `BoundExceeded` the same as
    /// every other re-encode this layer performs, rather than aborting the
    /// process.
    pub fn encode(&self) -> Result<Vec<u8>, ApplicationError> {
        encode_catalog_entries(&self.entries)
    }

    /// Strictly decode one closed canonical V1 catalog.
    ///
    /// # Why this uses the looser of the two entry bounds
    ///
    /// This checks entry count against [`MAX_ENCODABLE_CATALOG_ENTRIES`],
    /// not the tighter admission-time [`MAX_CATALOG_ENTRIES`]. The two
    /// differ by design: [`MAX_ENCODABLE_CATALOG_ENTRIES`] is a *proven*
    /// ceiling — with one candidate per entry, the cheapest an entry can
    /// ever encode to, no valid `CatalogV1::encode` output from any device
    /// honouring this wire format can exceed it — so rejecting past it
    /// costs nothing for any honestly-produced catalog, old client or new.
    /// Using the tighter, margined `MAX_CATALOG_ENTRIES` here instead would
    /// repeat the mistake VLT-PM05 §13.3 already corrected once for
    /// individual records: this decode runs on the open path (through
    /// `materialize_current_catalog`), and a hard reject there does not
    /// refuse one mutation, it denies the whole vault. A catalog a past
    /// version of this device (or any honest peer) legitimately produced
    /// between the two bounds must stay openable even though this device
    /// would no longer *admit* growing a catalog that large; only a
    /// catalog no honest encoder could ever have produced — because it
    /// exceeds the proven ceiling — is rejected here, which is exactly the
    /// case of a peer installing a catalog this device could never itself
    /// re-encode.
    pub fn decode(encoded: &[u8]) -> Result<Self, ApplicationError> {
        check_plaintext_bound(encoded)?;
        let mut fields = closed_fields(encoded, &[1, 2, 3])?;
        check_version(take_uint(&mut fields, 1)?)?;
        check_kind(take_uint(&mut fields, 2)?, ObjectKind::Catalog)?;
        let encoded_entries = take_array(&mut fields, 3)?;
        if encoded_entries.len() > MAX_ENCODABLE_CATALOG_ENTRIES {
            return Err(ApplicationError::BoundExceeded);
        }
        let mut entries = BTreeMap::new();
        let mut previous = None;
        for value in encoded_entries {
            let mut entry = value_fields(value, &[1, 2])?;
            let item = ItemId::new(take_fixed(&mut entry, 1)?);
            if previous.is_some_and(|prior| prior >= item) {
                return Err(ApplicationError::IntegrityFailure);
            }
            previous = Some(item);
            let candidate_values = take_array(&mut entry, 2)?;
            if candidate_values.is_empty() || candidate_values.len() > MAX_CANDIDATES_PER_ITEM {
                return Err(ApplicationError::IntegrityFailure);
            }
            let mut candidates = Vec::with_capacity(candidate_values.len());
            for candidate in candidate_values {
                let id = RevisionId::new(fixed_value(candidate)?);
                if candidates.last().is_some_and(|prior| prior >= &id) {
                    return Err(ApplicationError::IntegrityFailure);
                }
                candidates.push(id);
            }
            entries.insert(item, candidates);
        }
        // Not `Self::new`: that runs `validate_catalog`, which applies the
        // *admission* ceiling. The structural checks above (sorted, unique,
        // non-empty, within `MAX_CANDIDATES_PER_ITEM`) are exactly what
        // `validate_catalog` would otherwise re-check per entry; the only
        // thing it would add is the tighter entry-count bound this function
        // deliberately does not apply. See the decode doc comment above.
        Ok(Self { entries })
    }
}

/// Encode `entries` into the exact closed canonical V1 catalog body,
/// without checking entry count against either bound.
///
/// Factored out of [`CatalogV1::encode`] so tests can construct catalog
/// bytes past the admission ceiling — modelling a peer whose own
/// validation is looser than this device's — without reaching into the
/// wire format by hand.
pub(crate) fn encode_catalog_entries(
    entries: &BTreeMap<ItemId, Vec<RevisionId>>,
) -> Result<Vec<u8>, ApplicationError> {
    let encoded_entries = entries
        .iter()
        .map(|(item, candidates)| {
            CborValue::Map(vec![
                field(1, bytes(item.as_bytes())),
                field(
                    2,
                    CborValue::Array(
                        candidates
                            .iter()
                            .map(|candidate| bytes(candidate.as_bytes()))
                            .collect(),
                    ),
                ),
            ])
        })
        .collect();
    let encoded = try_encode(&CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(ObjectKind::Catalog.code())),
        field(3, CborValue::Array(encoded_entries)),
    ]))
    .map_err(map_encode_error)?;
    check_plaintext_bound(&encoded)?;
    Ok(encoded)
}

impl Debug for CatalogV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CatalogV1")
            .field("entry_count", &self.entries.len())
            .finish()
    }
}

/// Encode one lossless item revision application object.
///
/// # Two bounds, not one
///
/// `check_plaintext_bound` below is this layer's 16 MiB gate. It is the
/// *outer* of two bounds: canonical-CBOR independently refuses to emit
/// any single value past its own 1 MiB `MAX_ENCODED_SIZE`, and that
/// inner bound is the tighter one.
///
/// The inner bound bites here even when the record itself cleared it in
/// `encode_any_record`, because this map adds the item id, schema tag,
/// both timestamps, the favourite register, three observed sets, and
/// the causal-parent list *on top of* the record bytes. A record just
/// under the ceiling plus that framing lands just over it — so this is
/// a genuinely separate failure point from the record encode, not a
/// second guard on the same one.
///
/// The encode is therefore checked and reported as `BoundExceeded`.
/// The alternative was aborting the process on a record an untrusted
/// peer is free to author, which would take every later command
/// against the same vault down with it.
pub fn encode_item_revision(
    causal_parents: &BTreeSet<RevisionId>,
    item_state: &ItemState,
) -> Result<Vec<u8>, ApplicationError> {
    let state = match item_state {
        ItemState::Live(document) => encode_live(document)?,
        ItemState::Tombstone(tombstone) => CborValue::Map(vec![
            field(1, bytes(tombstone.item_id.as_bytes())),
            field(2, CborValue::Unsigned(tombstone.deleted_at_ms)),
        ]),
    };
    let state_code = match item_state {
        ItemState::Live(_) => LIVE_STATE,
        ItemState::Tombstone(_) => TOMBSTONE_STATE,
    };
    let encoded = try_encode(&CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(ObjectKind::ItemRevision.code())),
        field(
            3,
            CborValue::Array(
                causal_parents
                    .iter()
                    .map(|parent| bytes(parent.as_bytes()))
                    .collect(),
            ),
        ),
        field(4, CborValue::Unsigned(state_code)),
        field(5, state),
    ]))
    .map_err(map_encode_error)?;
    check_plaintext_bound(&encoded)?;
    Ok(encoded)
}

/// Strictly decode a lossless item revision using its authenticated frame ID.
pub fn decode_item_revision(
    revision_id: RevisionId,
    encoded: &[u8],
) -> Result<ItemCandidate, ApplicationError> {
    check_plaintext_bound(encoded)?;
    let mut fields = closed_fields(encoded, &[1, 2, 3, 4, 5])?;
    check_version(take_uint(&mut fields, 1)?)?;
    check_kind(take_uint(&mut fields, 2)?, ObjectKind::ItemRevision)?;
    let parents = take_array(&mut fields, 3)?
        .into_iter()
        .map(fixed_value)
        .map(|value| value.map(RevisionId::new))
        .collect::<Result<Vec<_>, _>>()?;
    if parents.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ApplicationError::IntegrityFailure);
    }
    let state_code = take_uint(&mut fields, 4)?;
    let state_value = fields
        .remove(&5)
        .ok_or(ApplicationError::IntegrityFailure)?;
    let state = match state_code {
        LIVE_STATE => ItemState::Live(Box::new(decode_live(state_value)?)),
        TOMBSTONE_STATE => {
            let mut tombstone = value_fields(state_value, &[1, 2])?;
            ItemState::Tombstone(Tombstone {
                item_id: ItemId::new(take_fixed(&mut tombstone, 1)?),
                deleted_at_ms: take_uint(&mut tombstone, 2)?,
            })
        }
        _ => return Err(ApplicationError::Unsupported),
    };
    ItemCandidate::new(revision_id, parents, state).map_err(map_domain)
}

/// Wrap one exact authority-signed VLT-PM01 device certificate as a canonical
/// authenticated application object.
pub fn encode_device_certificate(
    certificate: &DeviceCertificateV1,
) -> Result<Vec<u8>, ApplicationError> {
    encode_signed_object(
        ObjectKind::DeviceCertificate,
        certificate
            .encode()
            .map_err(|_| ApplicationError::IntegrityFailure)?,
    )
}

/// Strictly unwrap and decode one authority-signed VLT-PM01 device certificate.
pub fn decode_device_certificate(encoded: &[u8]) -> Result<DeviceCertificateV1, ApplicationError> {
    let exact = decode_signed_object(encoded, ObjectKind::DeviceCertificate)?;
    DeviceCertificateV1::decode(&exact).map_err(|_| ApplicationError::IntegrityFailure)
}

/// Wrap one exact device-signed VLT-PM01 commit as a canonical authenticated
/// application object.
pub fn encode_signed_commit(commit: &CommitV1) -> Result<Vec<u8>, ApplicationError> {
    encode_signed_object(
        ObjectKind::Commit,
        commit
            .encode()
            .map_err(|_| ApplicationError::IntegrityFailure)?,
    )
}

/// Strictly unwrap and decode one device-signed VLT-PM01 commit.
pub fn decode_signed_commit(encoded: &[u8]) -> Result<CommitV1, ApplicationError> {
    let exact = decode_signed_object(encoded, ObjectKind::Commit)?;
    CommitV1::decode(&exact).map_err(|_| ApplicationError::IntegrityFailure)
}

/// Wrap one exact device-signed VLT-PM15 operation event as a canonical
/// authenticated application object.
pub fn encode_signed_audit_event(event: &SignedAuditEventV1) -> Result<Vec<u8>, ApplicationError> {
    encode_signed_object(ObjectKind::AuditEvent, event.encode())
}

/// Strictly unwrap and decode one device-signed VLT-PM15 operation event.
///
/// This proves the encrypted object's canonical shape. Callers must separately
/// verify the event signature against the certified device public key.
pub fn decode_signed_audit_event(encoded: &[u8]) -> Result<SignedAuditEventV1, ApplicationError> {
    let exact = decode_signed_object(encoded, ObjectKind::AuditEvent)?;
    SignedAuditEventV1::decode(&exact).map_err(|_| ApplicationError::IntegrityFailure)
}

/// One attachment's metadata, key, and ordered chunk references.
///
/// VLT-PM47 §4.3. This is the object that lets an attachment of any permitted
/// size cost the item revision a fixed forty-eight bytes.
///
/// The attachment id here is also the VLT14 blob id — one 128-bit value with
/// both meanings, so the chunk AEAD's associated data binds each chunk to the
/// attachment identity a person sees rather than to a private alias, and there
/// is no state in which the two disagree.
pub struct AttachmentManifestV1 {
    attachment_id: AttachmentId,
    dek: Zeroizing<[u8; ATTACHMENT_DEK_BYTES]>,
    name: String,
    total_plaintext_len: u64,
    content_sha256: [u8; 32],
    chunks: Vec<ObjectId>,
    created_at_ms: u64,
}

impl AttachmentManifestV1 {
    /// Validate and construct one complete manifest.
    pub fn new(
        attachment_id: AttachmentId,
        dek: Zeroizing<[u8; ATTACHMENT_DEK_BYTES]>,
        name: String,
        total_plaintext_len: u64,
        content_sha256: [u8; 32],
        chunks: Vec<ObjectId>,
        created_at_ms: u64,
    ) -> Result<Self, ApplicationError> {
        let manifest = Self {
            attachment_id,
            dek,
            name,
            total_plaintext_len,
            content_sha256,
            chunks,
            created_at_ms,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Return the attachment identity, which is also the VLT14 blob id.
    pub const fn attachment_id(&self) -> AttachmentId {
        self.attachment_id
    }

    /// Borrow the per-attachment VLT14 data-encryption key.
    ///
    /// The key is plaintext only inside this object's authenticated payload,
    /// which the repository envelope seals under the vault's object-wrap key.
    /// VLT-PM00 §8.1 places attachment DEKs exactly there.
    pub const fn dek(&self) -> &Zeroizing<[u8; ATTACHMENT_DEK_BYTES]> {
        &self.dek
    }

    /// Borrow the validated base name the file was attached under.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the declared plaintext length.
    pub const fn total_plaintext_len(&self) -> u64 {
        self.total_plaintext_len
    }

    /// Borrow the SHA-256 of the complete plaintext.
    pub const fn content_sha256(&self) -> &[u8; 32] {
        &self.content_sha256
    }

    /// Borrow the chunk object references in chunk-index order.
    pub fn chunks(&self) -> &[ObjectId] {
        &self.chunks
    }

    /// Return the advisory creation timestamp.
    pub const fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    /// Enforce every V1 bound the format itself can decide.
    ///
    /// The chunk count and the declared length are checked against each other
    /// as well as against their ceilings, because a manifest whose length does
    /// not fit its chunk count is a manifest one of whose two numbers is a
    /// lie, and the reassembler should never have to decide which.
    fn validate(&self) -> Result<(), ApplicationError> {
        validate_attachment_name(&self.name)?;
        if self.total_plaintext_len > MAX_ATTACHMENT_BYTES as u64
            || self.chunks.len() > MAX_ATTACHMENT_CHUNKS
        {
            return Err(ApplicationError::BoundExceeded);
        }
        if self.chunks.is_empty() {
            return Err(ApplicationError::IntegrityFailure);
        }
        let expected = expected_chunk_count(self.total_plaintext_len);
        if self.chunks.len() != expected {
            return Err(ApplicationError::IntegrityFailure);
        }
        // Two identical chunk references cannot occur: every chunk carries a
        // distinct VLT14 nonce and is sealed under a distinct random object
        // DEK, so identical framed ciphertexts are not producible.
        let unique = self.chunks.iter().collect::<BTreeSet<_>>();
        if unique.len() != self.chunks.len() {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(())
    }

    /// Encode the exact closed canonical V1 application object.
    ///
    /// # Why this one returns a wiping buffer, and wipes its own scaffolding
    ///
    /// The output carries the per-attachment key, so it is the only encode in
    /// this module whose *result* is a secret. It returns `Zeroizing` so a
    /// caller cannot forget, and it wipes the intermediate CBOR tree before
    /// dropping it: `bytes(...)` copies the key into a fresh heap `Vec` that
    /// `try_encode` would otherwise free unwiped. VLT02 records that the CBOR
    /// value types are not zeroize-aware and that making them so is a change
    /// to a deliberately zero-dependency crate; this is the local half of that
    /// gap that can be closed without it.
    pub fn encode(&self) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        self.validate()?;
        let mut value = CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(
                2,
                CborValue::Unsigned(ObjectKind::AttachmentManifest.code()),
            ),
            field(3, bytes(self.attachment_id.as_bytes())),
            field(4, bytes(self.dek.as_slice())),
            field(5, CborValue::text(&self.name)),
            field(6, CborValue::Unsigned(self.total_plaintext_len)),
            field(7, bytes(&self.content_sha256)),
            field(
                8,
                CborValue::Array(
                    self.chunks
                        .iter()
                        .map(|chunk| bytes(chunk.as_bytes()))
                        .collect(),
                ),
            ),
            field(9, CborValue::Unsigned(self.created_at_ms)),
        ]);
        let encoded = try_encode(&value).map(Zeroizing::new);
        zeroize_cbor_secrets(&mut value);
        drop(value);
        let encoded = encoded.map_err(map_encode_error)?;
        check_plaintext_bound(&encoded)?;
        Ok(encoded)
    }

    /// Strictly decode one closed canonical V1 manifest.
    ///
    /// Every bound is applied before a buffer is sized from a declared value.
    /// A peer is free to author this object, so its numbers are input.
    ///
    /// # Every exit wipes the decoded fields, not just the successful one
    ///
    /// The decoded field map holds the per-attachment key until
    /// `take_secret_fixed` removes it, and *three* of the checks in front of
    /// that point can fail — a wrong version, a wrong kind, a malformed
    /// attachment id — each of them reachable from a manifest a synchronising
    /// peer authored. An early return there drops the map, and with it the key,
    /// unwiped.
    ///
    /// The body is therefore split out and its result funnelled through one
    /// place that wipes. That is structural: a check added to `decode_fields`
    /// tomorrow is covered without anyone remembering to cover it, which is
    /// the property a scatter of `zeroize` calls at each `return` would not
    /// have.
    pub fn decode(encoded: &[u8]) -> Result<Self, ApplicationError> {
        let mut fields = closed_fields(encoded, &[1, 2, 3, 4, 5, 6, 7, 8, 9])?;
        let decoded = Self::decode_fields(&mut fields);
        for value in fields.values_mut() {
            zeroize_cbor_secrets(value);
        }
        decoded
    }

    fn decode_fields(fields: &mut BTreeMap<u64, CborValue>) -> Result<Self, ApplicationError> {
        check_version(take_uint(fields, 1)?)?;
        check_kind(take_uint(fields, 2)?, ObjectKind::AttachmentManifest)?;
        let attachment_id = AttachmentId::new(take_fixed(fields, 3)?);
        let dek = take_secret_fixed::<ATTACHMENT_DEK_BYTES>(fields, 4)?;
        let name = take_text(fields, 5)?;
        let total_plaintext_len = take_uint(fields, 6)?;
        let content_sha256 = take_fixed(fields, 7)?;
        let chunk_values = take_array(fields, 8)?;
        if chunk_values.len() > MAX_ATTACHMENT_CHUNKS {
            return Err(ApplicationError::BoundExceeded);
        }
        let mut chunks = Vec::with_capacity(chunk_values.len());
        for chunk in chunk_values {
            chunks.push(ObjectId::new(fixed_value(chunk)?));
        }
        let created_at_ms = take_uint(fields, 9)?;
        Self::new(
            attachment_id,
            dek,
            name,
            total_plaintext_len,
            content_sha256,
            chunks,
            created_at_ms,
        )
    }
}

impl Zeroize for AttachmentManifestV1 {
    fn zeroize(&mut self) {
        self.dek.zeroize();
        self.name.zeroize();
    }
}

impl Drop for AttachmentManifestV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for AttachmentManifestV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AttachmentManifestV1")
            .field("attachment_id", &"<redacted>")
            .field("dek", &"<redacted>")
            .field("name", &"<redacted>")
            .field("total_plaintext_len", &self.total_plaintext_len)
            .field("chunk_count", &self.chunks.len())
            .finish()
    }
}

/// Return how many 64 KiB chunks a plaintext of `length` bytes occupies.
///
/// Zero-length attachments are refused at ingest, so every legal length is at
/// least one chunk; the `max(1)` exists so the relation this function states
/// is total rather than conditional.
pub(crate) fn expected_chunk_count(length: u64) -> usize {
    let chunks = length.div_ceil(ATTACHMENT_CHUNK_BYTES as u64).max(1);
    usize::try_from(chunks).unwrap_or(usize::MAX)
}

/// Wipe every byte string in one CBOR value in place.
///
/// Used on a tree that was built around a secret and is about to be dropped.
/// It wipes every byte string rather than only the one that is the key,
/// because a function that had to be told *which* field to wipe would be a
/// function somebody could forget to update when a field moved.
fn zeroize_cbor_secrets(value: &mut CborValue) {
    match value {
        CborValue::Bytes(bytes) => bytes.zeroize(),
        // Text as well as bytes. A record's password arrives here as a UTF-8
        // string inside a byte-string payload, but the item revision's tags
        // and content type are text at this layer, and a codec helper that
        // wiped only one of the two heap-owning variants would be a helper
        // whose guarantee depended on which variant a future field happened to
        // use.
        CborValue::Text(text) => text.zeroize(),
        CborValue::Array(values) => values.iter_mut().for_each(zeroize_cbor_secrets),
        CborValue::Map(entries) => entries.iter_mut().for_each(|(key, value)| {
            zeroize_cbor_secrets(key);
            zeroize_cbor_secrets(value);
        }),
        CborValue::Tag(_, inner) => zeroize_cbor_secrets(inner),
        _ => {}
    }
}

/// Take one fixed-width secret field, wiping the decoder's own copy of it.
///
/// `take_fixed` converts the decoded `Vec<u8>` into an array and drops the
/// vector, leaving the bytes in freed heap. For a field that is a key, that
/// residue is the thing the surrounding `Zeroizing`, `Drop` and redacted
/// `Debug` exist to prevent, so the vector is wiped before it goes.
fn take_secret_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Zeroizing<[u8; N]>, ApplicationError> {
    let mut value = fields
        .remove(&key)
        .ok_or(ApplicationError::IntegrityFailure)?;
    let taken = match &value {
        CborValue::Bytes(raw) => <[u8; N]>::try_from(raw.as_slice())
            .map(Zeroizing::new)
            .map_err(|_| ApplicationError::IntegrityFailure),
        _ => Err(ApplicationError::IntegrityFailure),
    };
    zeroize_cbor_secrets(&mut value);
    taken
}

/// Validate one attachment base name.
///
/// Rejected rather than repaired, for VLT-PM47 §4.5's reason: a sanitiser is a
/// function whose output the author did not choose, and the interesting inputs
/// to one are exactly the hostile ones. Nothing in this product turns a stored
/// name into a filesystem path (§4.6), so this is a well-formedness check and
/// not the traversal defence.
///
/// # This runs on decode too, and that is the case that matters
///
/// A name reaches this function twice: once from the operator's own filesystem
/// at ingest, and once out of a manifest a **synchronising peer** may have
/// authored. The second is the reason the character set below is wider than
/// `char::is_control`.
///
/// `is_control` is Unicode category Cc only. It blocks ESC, so a terminal
/// escape sequence cannot get through, and it blocks tab and newline, so the
/// tab-separated listing cannot be forged. It does **not** block category Cf —
/// U+202E RIGHT-TO-LEFT OVERRIDE, the directional isolates, the zero-width
/// space, the byte-order mark — nor the Zl/Zp line and paragraph separators. A
/// name carrying those renders as a *different* name, and the operator chooses
/// which attachment to export by reading that rendering. So they are refused
/// here as well.
///
/// The rendering layer escapes independently (`quoted` in the CLI). Two gates,
/// because a validator is not allowed to be the only thing standing between
/// peer-authored text and a terminal.
pub(crate) fn validate_attachment_name(name: &str) -> Result<(), ApplicationError> {
    if name.is_empty() || name.len() > MAX_ATTACHMENT_NAME_BYTES {
        return Err(ApplicationError::InvalidInput);
    }
    if name == "." || name == ".." {
        return Err(ApplicationError::InvalidInput);
    }
    if name.chars().any(is_forbidden_in_attachment_name) {
        return Err(ApplicationError::InvalidInput);
    }
    Ok(())
}

/// Whether one character may never appear in a stored attachment name.
///
/// # What this list is, and what it is not
///
/// It is an enumeration, so it is a *statement about the characters named in
/// it* and not a categorical guarantee about Unicode `Cf`. Every code point
/// below is one that hides, reorders, or invisibly splits the text around it,
/// and each is refused at the boundary so such a name is never stored in the
/// first place. But `Cf` gains members with each Unicode revision, and a
/// hand-written range list cannot promise to have them all.
///
/// **The total gate is the escape at the render site**, not this function.
/// `attachment list` renders every stored name through the CLI's `quoted`
/// helper, which is `str::escape_debug`, which escapes on the `char` property
/// rather than on a list somebody maintained — including the code points this
/// enumeration has not heard of. Reject-at-ingest is defence in depth on top
/// of that, and it is worth having because a stored name is also read by
/// future surfaces that have not been written yet.
///
/// Stated plainly so nobody later reads this list as the thing keeping a
/// terminal safe and deletes the escaping as redundant.
const fn is_forbidden_in_attachment_name(character: char) -> bool {
    character.is_control()
        || character == '/'
        || character == '\\'
        || matches!(
            character,
            // Cf: the soft hyphen, which renders as nothing or as a hyphen
            // depending entirely on the renderer.
            '\u{00AD}'
                // Arabic and Syriac formatting marks, including the Arabic
                // letter mark and the number/letter shaping controls.
                | '\u{0600}'..='\u{0605}'
                | '\u{061C}'
                | '\u{06DD}'
                | '\u{070F}'
                | '\u{0890}'..='\u{0891}'
                | '\u{08E2}'
                // The Mongolian vowel separator.
                | '\u{180E}'
                // Zero-width characters, the bidirectional marks, and the
                // overrides and embeddings that reorder what follows.
                | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2060}'..='\u{2064}'
                | '\u{2066}'..='\u{206F}'
                // The byte-order mark and the interlinear annotation controls.
                | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}'
                // Egyptian hieroglyph and Kaithi formatting controls, and the
                // Brahmi number joiner.
                | '\u{110BD}'
                | '\u{110CD}'
                | '\u{13430}'..='\u{1343F}'
                | '\u{1BCA0}'..='\u{1BCA3}'
                | '\u{1D173}'..='\u{1D17A}'
                // The deprecated tag block, which encodes an entire invisible
                // ASCII string inside one apparent character run.
                | '\u{E0001}'
                | '\u{E0020}'..='\u{E007F}'
                // Zl and Zp: a line and a paragraph separator, which some
                // renderers honour as a break and which would let one stored
                // name look like two listing rows.
                | '\u{2028}'
                | '\u{2029}'
        )
}

/// Encode one VLT14 sealed chunk as a canonical application object.
///
/// The encoded value is about 65,600 bytes and cannot grow with the file,
/// which is the entire reason attachments are chunked: canonical-CBOR refuses
/// to emit any single value past 1 MiB (VLT-PM47 §3).
pub fn encode_attachment_chunk(chunk: &EncryptedChunk) -> Result<Vec<u8>, ApplicationError> {
    if chunk.ciphertext.len() > ATTACHMENT_CHUNK_BYTES
        || chunk.index as usize >= MAX_ATTACHMENT_CHUNKS
    {
        return Err(ApplicationError::BoundExceeded);
    }
    let encoded = try_encode(&CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(ObjectKind::AttachmentChunk.code())),
        field(3, bytes(chunk.blob_id.as_bytes())),
        field(4, CborValue::Unsigned(u64::from(chunk.index))),
        field(5, CborValue::Bool(chunk.is_final)),
        field(6, bytes(&chunk.ciphertext)),
        field(7, bytes(&chunk.tag)),
    ]))
    .map_err(map_encode_error)?;
    check_plaintext_bound(&encoded)?;
    Ok(encoded)
}

/// Strictly decode one canonical application object into a VLT14 chunk.
///
/// The bounds here mirror the encoder's rather than trusting it: this object
/// can be authored by a synchronising peer, and VLT14's decryptor is the next
/// gate rather than the first one.
pub fn decode_attachment_chunk(encoded: &[u8]) -> Result<EncryptedChunk, ApplicationError> {
    let mut fields = closed_fields(encoded, &[1, 2, 3, 4, 5, 6, 7])?;
    check_version(take_uint(&mut fields, 1)?)?;
    check_kind(take_uint(&mut fields, 2)?, ObjectKind::AttachmentChunk)?;
    let blob_id = BlobId(take_fixed(&mut fields, 3)?);
    let index = take_uint(&mut fields, 4)?;
    if index >= MAX_ATTACHMENT_CHUNKS as u64 {
        return Err(ApplicationError::BoundExceeded);
    }
    let is_final = take_bool(&mut fields, 5)?;
    let ciphertext = take_bytes(&mut fields, 6)?;
    if ciphertext.len() > ATTACHMENT_CHUNK_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    let tag = take_fixed(&mut fields, 7)?;
    Ok(EncryptedChunk {
        blob_id,
        index: u32::try_from(index).map_err(|_| ApplicationError::BoundExceeded)?,
        is_final,
        ciphertext,
        tag,
    })
}

fn encode_signed_object(kind: ObjectKind, exact: Vec<u8>) -> Result<Vec<u8>, ApplicationError> {
    // `exact` is a caller-supplied encoded object, so its length is not
    // statically bounded by anything tighter than the 16 MiB plaintext
    // gate below — which is looser than the encoder's own 1 MiB ceiling.
    // Checked for the same reason as the record and catalog encodes.
    let encoded = try_encode(&CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(kind.code())),
        field(3, CborValue::Bytes(exact)),
    ]))
    .map_err(map_encode_error)?;
    check_plaintext_bound(&encoded)?;
    Ok(encoded)
}

fn decode_signed_object(
    encoded: &[u8],
    expected_kind: ObjectKind,
) -> Result<Vec<u8>, ApplicationError> {
    let mut fields = closed_fields(encoded, &[1, 2, 3])?;
    check_version(take_uint(&mut fields, 1)?)?;
    check_kind(take_uint(&mut fields, 2)?, expected_kind)?;
    take_bytes(&mut fields, 3)
}

/// Encode one live item document.
///
/// # The tenth field
///
/// Key `10` carries the manifest object reference for every retained
/// attachment, and it is emitted **only** when there is at least one. That
/// conditional is the whole compatibility story of VLT-PM47 §4.7: an item with
/// no attachments encodes exactly the nine keys this function emitted before
/// attachments existed, byte for byte, so every revision written by every
/// earlier version of this product still decodes and every revision this
/// product writes for an unattached item is unchanged.
///
/// The reference list is *derived* from `document.attachment_manifests()`,
/// which VLT-PM03 already forces to have exactly the key set of
/// `attachments.retained_values()`. There is no encoding of a disagreement.
fn encode_live(document: &ItemDocument) -> Result<CborValue, ApplicationError> {
    let record = encode_any_record(document.payload())?;
    let mut entries = vec![
        field(1, bytes(document.id().as_bytes())),
        field(2, CborValue::text(document.schema().as_str())),
        field(3, CborValue::Unsigned(document.created_at_ms())),
        field(4, CborValue::Unsigned(document.updated_at_ms())),
        field(
            5,
            CborValue::Map(vec![
                field(1, CborValue::Bool(*document.favorite().value())),
                field(2, CborValue::Unsigned(document.favorite().updated_at_ms())),
                field(3, bytes(document.favorite().operation().as_bytes())),
            ]),
        ),
        field(6, encode_observed(document.collection_ids())),
        field(7, encode_observed(document.tags())),
        field(8, CborValue::Bytes(record)),
        field(9, encode_observed(document.attachments())),
    ];
    if !document.attachment_manifests().is_empty() {
        entries.push(field(
            10,
            CborValue::Array(
                document
                    .attachment_manifests()
                    .iter()
                    .map(|(attachment, manifest)| {
                        CborValue::Map(vec![
                            field(1, bytes(attachment.as_bytes())),
                            field(2, bytes(manifest.as_bytes())),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    Ok(CborValue::Map(entries))
}

fn decode_live(value: CborValue) -> Result<ItemDocument, ApplicationError> {
    // Nine keys is the pre-attachment shape and ten is the attachment shape.
    // Any other arity, an unknown key, or a duplicate is still refused: this
    // is a closed decoder with two accepted key sets, not an open one.
    let expected: &[u64] = match &value {
        CborValue::Map(entries) if entries.len() == 10 => &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        _ => &[1, 2, 3, 4, 5, 6, 7, 8, 9],
    };
    let mut fields = value_fields(value, expected)?;
    let id = ItemId::new(take_fixed(&mut fields, 1)?);
    let schema = ContentType::new(take_text(&mut fields, 2)?).map_err(map_domain)?;
    let created_at_ms = take_uint(&mut fields, 3)?;
    let updated_at_ms = take_uint(&mut fields, 4)?;
    let mut favorite = value_fields(
        fields
            .remove(&5)
            .ok_or(ApplicationError::IntegrityFailure)?,
        &[1, 2, 3],
    )?;
    let favorite = LwwRegister::new(
        take_bool(&mut favorite, 1)?,
        take_uint(&mut favorite, 2)?,
        OperationId::new(take_fixed(&mut favorite, 3)?),
    );
    let collections = decode_observed(
        fields
            .remove(&6)
            .ok_or(ApplicationError::IntegrityFailure)?,
    )?;
    let tags = decode_observed(
        fields
            .remove(&7)
            .ok_or(ApplicationError::IntegrityFailure)?,
    )?;
    let record_bytes = take_bytes(&mut fields, 8)?;
    let payload = decode_record(&record_bytes).map_err(|_| ApplicationError::IntegrityFailure)?;
    let attachments = decode_observed(
        fields
            .remove(&9)
            .ok_or(ApplicationError::IntegrityFailure)?,
    )?;
    let attachment_manifests = match fields.remove(&10) {
        None => BTreeMap::new(),
        Some(value) => decode_attachment_manifest_references(value)?,
    };
    ItemDocument::new(
        id,
        schema,
        created_at_ms,
        updated_at_ms,
        favorite,
        collections,
        tags,
        payload,
        attachments,
        attachment_manifests,
    )
    .map_err(map_domain)
}

/// Decode the tenth live-state field's attachment-to-manifest references.
///
/// Bounded before anything is built, ascending and unique on the wire, and
/// checked for the key-set equality VLT-PM03 enforces — the domain
/// constructor would catch the last of those anyway, but reaching it requires
/// building the map first, and a peer-authored list is exactly the input that
/// should be refused before it is materialised.
fn decode_attachment_manifest_references(
    value: CborValue,
) -> Result<BTreeMap<AttachmentId, AttachmentManifestId>, ApplicationError> {
    let entries = match value {
        CborValue::Array(entries) => entries,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    if entries.is_empty() {
        // Absence is encoded by omitting the field, so an empty array is a
        // second spelling of the same state and canonical encodings have one.
        return Err(ApplicationError::IntegrityFailure);
    }
    if entries.len() > MAX_OBSERVED_VALUES {
        return Err(ApplicationError::BoundExceeded);
    }
    let mut references = BTreeMap::new();
    let mut previous: Option<AttachmentId> = None;
    for entry in entries {
        let mut fields = value_fields(entry, &[1, 2])?;
        let attachment = AttachmentId::new(take_fixed(&mut fields, 1)?);
        if previous.is_some_and(|prior| prior >= attachment) {
            return Err(ApplicationError::IntegrityFailure);
        }
        previous = Some(attachment);
        let manifest = AttachmentManifestId::new(take_fixed(&mut fields, 2)?);
        references.insert(attachment, manifest);
    }
    Ok(references)
}

trait ObservedValue: Ord + Clone {
    fn encode_value(&self) -> CborValue;
    fn decode_value(value: CborValue) -> Result<Self, ApplicationError>;
}

macro_rules! observed_id {
    ($name:ty, $size:expr) => {
        impl ObservedValue for $name {
            fn encode_value(&self) -> CborValue {
                bytes(self.as_bytes())
            }

            fn decode_value(value: CborValue) -> Result<Self, ApplicationError> {
                Ok(Self::new(fixed_value::<$size>(value)?))
            }
        }
    };
}

observed_id!(CollectionId, 16);
observed_id!(AttachmentId, 16);

impl ObservedValue for String {
    fn encode_value(&self) -> CborValue {
        CborValue::Text(self.clone())
    }

    fn decode_value(value: CborValue) -> Result<Self, ApplicationError> {
        match value {
            CborValue::Text(value) => Ok(value),
            _ => Err(ApplicationError::IntegrityFailure),
        }
    }
}

fn encode_observed<T: ObservedValue>(set: &ObservedSet<T>) -> CborValue {
    CborValue::Array(
        set.retained_values()
            .map(|value| {
                CborValue::Map(vec![
                    field(1, value.encode_value()),
                    field(
                        2,
                        CborValue::Array(
                            set.retained_add_operations(value)
                                .map(|operation| bytes(operation.as_bytes()))
                                .collect(),
                        ),
                    ),
                    field(
                        3,
                        CborValue::Array(
                            set.retained_removal_operations(value)
                                .map(|operation| bytes(operation.as_bytes()))
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}

fn decode_observed<T: ObservedValue>(value: CborValue) -> Result<ObservedSet<T>, ApplicationError> {
    let values = match value {
        CborValue::Array(values) => values,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    let mut result = ObservedSet::new();
    let mut previous: Option<T> = None;
    for value in values {
        let mut entry = value_fields(value, &[1, 2, 3])?;
        let item = T::decode_value(entry.remove(&1).ok_or(ApplicationError::IntegrityFailure)?)?;
        if previous.as_ref().is_some_and(|prior| prior >= &item) {
            return Err(ApplicationError::IntegrityFailure);
        }
        let adds = decode_operations(entry.remove(&2).ok_or(ApplicationError::IntegrityFailure)?)?;
        if adds.is_empty() {
            return Err(ApplicationError::IntegrityFailure);
        }
        let removals =
            decode_operations(entry.remove(&3).ok_or(ApplicationError::IntegrityFailure)?)?;
        for operation in adds {
            result.add(item.clone(), operation).map_err(map_domain)?;
        }
        for operation in removals {
            result
                .observe_removal(&item, operation)
                .map_err(map_domain)?;
        }
        previous = Some(item);
    }
    Ok(result)
}

fn decode_operations(value: CborValue) -> Result<Vec<OperationId>, ApplicationError> {
    let values = match value {
        CborValue::Array(values) => values,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    let mut operations = Vec::with_capacity(values.len());
    for value in values {
        let operation = OperationId::new(fixed_value(value)?);
        if operations.last().is_some_and(|prior| prior >= &operation) {
            return Err(ApplicationError::IntegrityFailure);
        }
        operations.push(operation);
    }
    Ok(operations)
}

/// Encode one record of any schema back to its canonical wire bytes.
///
/// # Why the first-party arms can fail
///
/// This crate's `MAX_PLAINTEXT_BYTES` is 16 MiB; canonical-CBOR's
/// `MAX_ENCODED_SIZE` is 1 MiB. The looser gate is ours, so a record
/// between the two is one we will happily *hold* and *decode* and the
/// encoder will nonetheless refuse to re-emit — see VLT02 "Encoding is
/// fallible" and VLT-PM05 section 13.1.
///
/// Such a record is reachable from an untrusted peer whose framing
/// budget is larger than the codec's ceiling, and it is re-encoded by
/// `item edit`, all seven authored conflict merges, `conflict choose`,
/// `history restore`, and `export`. Reporting `BoundExceeded` there
/// loses one record; the panicking encode this replaced lost the whole
/// process, repeatedly, because the record stays in the store.
///
/// `BoundExceeded` rather than `IntegrityFailure` because the record is
/// not corrupt — it is merely larger than a fixed serialisation bound
/// allows, which is precisely what `BoundExceeded` names.
///
/// Every arm, the opaque one included, routes through
/// [`map_record_encode_error`], so the size and integrity faults stay
/// distinct on all seven. The opaque arm used to fold every
/// `encode_opaque` failure into `IntegrityFailure` on the grounds that
/// its dominant failure is genuinely one — stored payload bytes that are
/// not valid CBOR at all — and while that reasoning still holds for the
/// dominant case, the size case is no longer unreachable. Before
/// `decode_record`'s opaque arm stopped re-encoding, an oversized opaque
/// record could never be materialised, so this arm could never see
/// `EncodeTooLarge` from stored bytes. Now that such a record opens, it
/// can, and reporting it as `IntegrityFailure` would tell an operator
/// their store is corrupt — inviting destructive recovery — when the
/// remedy is to delete one large item. That remedy *is* the escape hatch
/// VLT-PM05 §13.3 exists to restore, so it must not be described in the
/// vocabulary of corruption.
///
/// VLT-PM39's dependency is unaffected: a payload that is not valid CBOR
/// yields a non-size `CborError`, which `map_record_encode_error` still
/// routes to `IntegrityFailure`.
fn encode_any_record(record: &AnyRecord) -> Result<Vec<u8>, ApplicationError> {
    match record {
        AnyRecord::Login(value) => encode_record(value).map_err(map_record_encode_error),
        AnyRecord::SecureNote(value) => encode_record(value).map_err(map_record_encode_error),
        AnyRecord::Card(value) => encode_record(value).map_err(map_record_encode_error),
        AnyRecord::TotpSeed(value) => encode_record(value).map_err(map_record_encode_error),
        AnyRecord::ApiKey(value) => encode_record(value).map_err(map_record_encode_error),
        AnyRecord::DatabaseCredential(value) => {
            encode_record(value).map_err(map_record_encode_error)
        }
        AnyRecord::Opaque {
            content_type,
            payload_bytes,
        } => encode_opaque(content_type, payload_bytes).map_err(map_record_encode_error),
        // A quarantined record's `payload_bytes` are the inner payload's own
        // already-canonical CBOR bytes (VLT-PM05 §13.3's decode-time fix —
        // see `decode_record`'s doc comment in vault-records), the exact
        // same shape `Opaque` carries. Re-wrapping them with `encode_opaque`
        // forwards the record faithfully on write paths that walk every
        // live candidate (`export`, `history restore`, conflict merges):
        // still unreadable, still round-trips byte-for-byte, never silently
        // repaired or dropped.
        AnyRecord::Quarantined {
            content_type,
            payload_bytes,
            reason: _,
        } => encode_opaque(content_type, payload_bytes).map_err(map_record_encode_error),
    }
}

/// Admission-time validation for a catalog built entirely from scratch —
/// used by [`CatalogV1::new`]. Applies the tight, margined
/// [`MAX_CATALOG_ENTRIES`] to the *whole* entry count, which is correct
/// exactly when every entry is new growth (portable import, tests) and
/// wrong for an ordinary item mutation that carries forward entries that
/// already existed — see [`CatalogV1::new_for_mutation`] for that case.
fn validate_catalog(entries: &BTreeMap<ItemId, Vec<RevisionId>>) -> Result<(), ApplicationError> {
    if entries.len() > MAX_CATALOG_ENTRIES {
        return Err(ApplicationError::BoundExceeded);
    }
    validate_catalog_structure(entries)
}

/// Structural checks shared by every catalog construction path, regardless
/// of which entry-count ceiling applies: every item's candidate list is
/// non-empty, within [`MAX_CANDIDATES_PER_ITEM`], and sorted with no
/// duplicates.
fn validate_catalog_structure(
    entries: &BTreeMap<ItemId, Vec<RevisionId>>,
) -> Result<(), ApplicationError> {
    for candidates in entries.values() {
        if candidates.is_empty() || candidates.len() > MAX_CANDIDATES_PER_ITEM {
            return Err(ApplicationError::InvalidInput);
        }
        if candidates.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ApplicationError::InvalidInput);
        }
    }
    Ok(())
}

/// Map one canonical-CBOR *encode* failure into this crate's closed
/// taxonomy.
///
/// Only two variants mean "this value is too big or too deep to
/// represent", which is what `BoundExceeded` names. Every other variant
/// is a genuine canonicality or integrity fault — `DuplicateMapKey`
/// most of all, since it says the value would encode to bytes the strict
/// decoder rejects as ambiguous — and must not be laundered into a
/// benign size error.
///
/// The wildcard this replaced collapsed all of them together. That was
/// unreachable in practice, because every map this crate builds has
/// distinct literal keys, but it was a trap: a later refactor that did
/// introduce a duplicate key would have reported a real integrity fault
/// as a size problem, and the size problem is the one an operator is
/// told to fix by shrinking a record.
pub(crate) fn map_encode_error(error: CborError) -> ApplicationError {
    match error {
        CborError::EncodeTooLarge | CborError::EncodeTooDeep => ApplicationError::BoundExceeded,
        _ => ApplicationError::IntegrityFailure,
    }
}

/// The same mapping one layer up, for the record codec's own error type.
fn map_record_encode_error(error: VaultRecordError) -> ApplicationError {
    match error {
        VaultRecordError::Cbor(inner) => map_encode_error(inner),
        _ => ApplicationError::IntegrityFailure,
    }
}

fn check_plaintext_bound(encoded: &[u8]) -> Result<(), ApplicationError> {
    if encoded.len() > MAX_PLAINTEXT_BYTES {
        Err(ApplicationError::BoundExceeded)
    } else {
        Ok(())
    }
}

fn closed_fields(
    encoded: &[u8],
    expected: &[u64],
) -> Result<BTreeMap<u64, CborValue>, ApplicationError> {
    check_plaintext_bound(encoded)?;
    let value = decode(encoded).map_err(|_| ApplicationError::IntegrityFailure)?;
    value_fields(value, expected)
}

/// Split one closed canonical CBOR map into its exact expected fields.
///
/// # A rejected map is wiped before it is dropped
///
/// This function is the single gate every object in this crate decodes
/// through, and several of those objects are made of key material: the
/// attachment manifest's per-attachment key, and `LocalSecretV1`'s authority
/// and device seeds. On the failure paths — not a map, wrong arity, an
/// unexpected key, a duplicate key, every one of them reachable from a value a
/// synchronising peer authored — the entries decoded so far were simply
/// dropped, leaving those bytes in freed heap.
///
/// So every exit funnels through one place that wipes both halves of the
/// work in progress: the entries not yet consumed and the fields already
/// collected. It costs a `memset` on a path that is about to return an error
/// anyway, and it means a caller cannot leak by forgetting, which is the same
/// reason [`AttachmentManifestV1::decode`] is shaped the way it is.
fn value_fields(
    value: CborValue,
    expected: &[u64],
) -> Result<BTreeMap<u64, CborValue>, ApplicationError> {
    let mut entries = match value {
        CborValue::Map(entries) => entries,
        mut other => {
            zeroize_cbor_secrets(&mut other);
            return Err(ApplicationError::IntegrityFailure);
        }
    };
    let mut fields = BTreeMap::new();
    match collect_value_fields(&mut entries, expected, &mut fields) {
        Ok(()) => Ok(fields),
        Err(error) => {
            for (key, value) in &mut entries {
                zeroize_cbor_secrets(key);
                zeroize_cbor_secrets(value);
            }
            for value in fields.values_mut() {
                zeroize_cbor_secrets(value);
            }
            Err(error)
        }
    }
}

/// Move entries into `fields`, leaving whatever it did not reach in `entries`.
///
/// Draining rather than consuming by value is what lets the caller above wipe
/// the remainder: a `for` loop over an owned `Vec` drops the tail on an early
/// return with nothing holding it.
fn collect_value_fields(
    entries: &mut Vec<(CborValue, CborValue)>,
    expected: &[u64],
    fields: &mut BTreeMap<u64, CborValue>,
) -> Result<(), ApplicationError> {
    if entries.len() != expected.len() {
        return Err(ApplicationError::IntegrityFailure);
    }
    // Order is irrelevant to the result — the output is keyed by integer, and
    // canonical ordering was already enforced by the CBOR decoder — so popping
    // from the back is free and keeps the untouched entries owned by `entries`.
    while let Some((key, value)) = entries.pop() {
        match resolve_expected_key(key, expected) {
            Ok(key) => {
                if fields.insert(key, value).is_some() {
                    return Err(ApplicationError::IntegrityFailure);
                }
            }
            // The rejected key goes back too, not a placeholder. A key is a
            // `CborValue` like any other and a peer chooses what it is, so it
            // can be a byte string; handing back only the value would leave
            // that half to be dropped unwiped, which is the whole class of bug
            // this shape exists to close.
            Err(key) => {
                entries.push((key, value));
                return Err(ApplicationError::IntegrityFailure);
            }
        }
    }
    Ok(())
}

/// Resolve one map key against the closed expected set, returning it intact on
/// rejection so the caller can wipe it.
fn resolve_expected_key(key: CborValue, expected: &[u64]) -> Result<u64, CborValue> {
    match key {
        CborValue::Unsigned(value) if expected.contains(&value) => Ok(value),
        other => Err(other),
    }
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes(value: &[u8]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn take_uint(fields: &mut BTreeMap<u64, CborValue>, key: u64) -> Result<u64, ApplicationError> {
    match fields.remove(&key) {
        Some(CborValue::Unsigned(value)) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_bool(fields: &mut BTreeMap<u64, CborValue>, key: u64) -> Result<bool, ApplicationError> {
    match fields.remove(&key) {
        Some(CborValue::Bool(value)) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_text(fields: &mut BTreeMap<u64, CborValue>, key: u64) -> Result<String, ApplicationError> {
    match fields.remove(&key) {
        Some(CborValue::Text(value)) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_bytes(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Vec<u8>, ApplicationError> {
    match fields.remove(&key) {
        Some(CborValue::Bytes(value)) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<[u8; N], ApplicationError> {
    fixed_value(
        fields
            .remove(&key)
            .ok_or(ApplicationError::IntegrityFailure)?,
    )
}

fn fixed_value<const N: usize>(value: CborValue) -> Result<[u8; N], ApplicationError> {
    match value {
        CborValue::Bytes(value) => value
            .try_into()
            .map_err(|_| ApplicationError::IntegrityFailure),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_array(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Vec<CborValue>, ApplicationError> {
    match fields.remove(&key) {
        Some(CborValue::Array(values)) => Ok(values),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn check_version(version: u64) -> Result<(), ApplicationError> {
    if version == VERSION {
        Ok(())
    } else {
        Err(ApplicationError::Unsupported)
    }
}

fn check_kind(value: u64, expected: ObjectKind) -> Result<(), ApplicationError> {
    if value == expected.code() {
        Ok(())
    } else {
        Err(ApplicationError::IntegrityFailure)
    }
}

fn map_domain(error: coding_adventures_vault_pm_domain::DomainError) -> ApplicationError {
    match error {
        coding_adventures_vault_pm_domain::DomainError::BoundExceeded => {
            ApplicationError::BoundExceeded
        }
        _ => ApplicationError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_canonical_cbor::encode;
    use coding_adventures_ed25519::generate_keypair;
    use coding_adventures_vault_pm_audit::{AuditActionV1, AuditEventV1, AuditOutcomeV1};
    use coding_adventures_vault_pm_format::{ObjectId, PublicKey, Signature};
    use coding_adventures_vault_records::{Login, LOGIN_V1};

    fn op(value: u8) -> OperationId {
        OperationId::new([value; 32])
    }

    fn live_candidate() -> ItemCandidate {
        let mut collections = ObservedSet::new();
        let collection = CollectionId::new([0x31; 16]);
        collections.add(collection, op(1)).unwrap();
        collections.remove(&collection);
        collections.add(collection, op(2)).unwrap();

        let mut tags = ObservedSet::new();
        tags.add("personal".to_string(), op(3)).unwrap();
        tags.remove(&"personal".to_string());

        let mut attachments = ObservedSet::new();
        attachments
            .add(AttachmentId::new([0x41; 16]), op(4))
            .unwrap();
        // This fixture carries an attachment, so the round trip through this
        // module exercises the tenth live-state field rather than only the
        // nine-key shape.
        let manifests = BTreeMap::from([(
            AttachmentId::new([0x41; 16]),
            AttachmentManifestId::new([0x61; 32]),
        )]);

        let document = ItemDocument::new(
            ItemId::new([0x21; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            10,
            20,
            LwwRegister::new(true, 15, op(5)),
            collections,
            tags,
            AnyRecord::Login(Login {
                title: "Example".to_string(),
                username: "alice".to_string(),
                password: "correct horse".to_string(),
                urls: vec!["https://example.test".to_string()],
                notes: Some("private".to_string()),
            }),
            attachments,
            manifests,
        )
        .unwrap();
        ItemCandidate::new(
            RevisionId::new([0x51; 32]),
            [RevisionId::new([0x50; 32])],
            ItemState::Live(Box::new(document)),
        )
        .unwrap()
    }

    #[test]
    fn local_secret_is_canonical_round_trippable_and_redacted() {
        let secret = LocalSecretV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            [3; 32],
            [4; 32],
            [5; 32],
        );
        let encoded = secret.encode();
        assert_eq!(
            hex(&encoded),
            "a60101025001010101010101010101010101010101035002020202020202020202020202020202045820030303030303030303030303030303030303030303030303030303030303030305582004040404040404040404040404040404040404040404040404040404040404040658200505050505050505050505050505050505050505050505050505050505050505"
        );
        let decoded = LocalSecretV1::decode(&encoded).unwrap();
        assert_eq!(decoded.vault_id(), VaultId::new([1; 16]));
        assert_eq!(decoded.device_id(), DeviceId::new([2; 16]));
        assert_eq!(decoded.authority_seed(), &[3; 32]);
        assert_eq!(decoded.device_signing_seed(), &[4; 32]);
        assert_eq!(decoded.device_x25519_secret(), &[5; 32]);
        assert_eq!(format!("{decoded:?}"), "LocalSecretV1(<redacted>)");
        assert_eq!(encoded, decoded.encode());
    }

    #[test]
    fn local_secret_zeroize_and_strict_decode() {
        let mut secret = LocalSecretV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            [3; 32],
            [4; 32],
            [5; 32],
        );
        secret.zeroize();
        assert_eq!(secret.authority_seed(), &[0; 32]);
        assert_eq!(secret.device_signing_seed(), &[0; 32]);
        assert_eq!(secret.device_x25519_secret(), &[0; 32]);

        assert_eq!(
            LocalSecretV1::decode(&[]),
            Err(ApplicationError::IntegrityFailure)
        );
        let extra = encode(&CborValue::Map(vec![field(1, CborValue::Unsigned(1))]));
        assert_eq!(
            LocalSecretV1::decode(&extra),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    /// Inside this crate's own 16 MiB plaintext gate, outside
    /// canonical-CBOR's 1 MiB encoded ceiling. The shape a record
    /// authored by a peer with a larger framing budget takes.
    const HUGE_PASSWORD_BYTES: usize = 2 * 1024 * 1024;

    /// A live single-item candidate whose `Login.password` is `n` bytes
    /// and whose every other field is as small as the schema allows, so
    /// the encoded size is `n` plus a constant.
    fn candidate_with_password_len(n: usize) -> ItemCandidate {
        let document = ItemDocument::new(
            ItemId::new([0x21; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            10,
            20,
            LwwRegister::new(true, 15, op(5)),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: String::new(),
                username: String::new(),
                password: "a".repeat(n),
                urls: Vec::new(),
                notes: None,
            }),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        ItemCandidate::new(
            RevisionId::new([0x51; 32]),
            [RevisionId::new([0x50; 32])],
            ItemState::Live(Box::new(document)),
        )
        .unwrap()
    }

    /// Build `count` distinct, sorted, single-candidate catalog entries.
    fn many_single_candidate_entries(count: usize) -> BTreeMap<ItemId, Vec<RevisionId>> {
        let mut entries = BTreeMap::new();
        for index in 0..count {
            let index = u64::try_from(index).unwrap();
            let mut id = [0_u8; 16];
            id[..8].copy_from_slice(&index.to_be_bytes());
            entries.insert(ItemId::new(id), vec![RevisionId::new([1; 32])]);
        }
        entries
    }

    /// One CBOR definite-length header: major type `major`, argument `len`,
    /// smallest form. Independent of `try_encode` so tests can author bytes
    /// no honest use of this crate's own encoder could ever produce --
    /// modelling a peer whose encoder does not share `MAX_ENCODED_SIZE`.
    fn raw_cbor_header(major: u8, len: usize) -> Vec<u8> {
        let major = major << 5;
        if len <= 23 {
            vec![major | len as u8]
        } else if len <= 0xFF {
            vec![major | 24, len as u8]
        } else if len <= 0xFFFF {
            let mut out = vec![major | 25];
            out.extend_from_slice(&(len as u16).to_be_bytes());
            out
        } else {
            let mut out = vec![major | 26];
            out.extend_from_slice(&(len as u32).to_be_bytes());
            out
        }
    }

    /// Hand-assemble `count` catalog entries directly onto the wire,
    /// bypassing `try_encode` (and therefore `MAX_ENCODED_SIZE`) entirely.
    /// This is what a peer whose own encoder has a larger — or absent —
    /// framing budget would produce: valid canonical CBOR this device's own
    /// encoder could never have written.
    fn raw_catalog_bytes(count: usize) -> Vec<u8> {
        let mut out = vec![0xa3, 0x01, 0x01, 0x02, 0x02, 0x03];
        out.extend(raw_cbor_header(4, count));
        for index in 0..count {
            let index = u64::try_from(index).unwrap();
            let mut item_id = [0_u8; 16];
            item_id[..8].copy_from_slice(&index.to_be_bytes());
            out.extend([0xa2, 0x01]);
            out.extend(raw_cbor_header(2, 16));
            out.extend_from_slice(&item_id);
            out.extend([0x02, 0x81]);
            out.extend(raw_cbor_header(2, 32));
            out.extend_from_slice(&[1_u8; 32]);
        }
        out
    }

    #[test]
    fn catalog_entry_byte_cost_is_exact() {
        // `CATALOG_ENTRY_BYTES` is asserted, not measured, in the constant's
        // own doc comment. Pin it against the real encoder so a change to
        // `ItemId`/`RevisionId` width, or to the CBOR profile, fails this
        // test instead of silently invalidating every bound derived from it.
        let empty = encode_catalog_entries(&BTreeMap::new()).unwrap();
        let one = encode_catalog_entries(&many_single_candidate_entries(1)).unwrap();
        assert_eq!(one.len() - empty.len(), CATALOG_ENTRY_BYTES);

        // The hand-assembled wire form used by the tests below must be
        // byte-for-byte what the real encoder produces, or the "peer" bytes
        // those tests build would not be modelling this wire format at all.
        assert_eq!(raw_catalog_bytes(0), empty);
        assert_eq!(raw_catalog_bytes(1), one);
        assert_eq!(
            raw_catalog_bytes(500),
            encode_catalog_entries(&many_single_candidate_entries(500)).unwrap(),
        );
    }

    #[test]
    fn max_encodable_catalog_entries_is_a_proven_ceiling() {
        // No valid encode of any catalog, from this device or any other
        // honouring MAX_ENCODED_SIZE, can exceed this many entries: one
        // candidate per entry is the cheapest an entry can be, so this is
        // the theoretical maximum, not an estimate. Proven two ways: the
        // real encoder accepts exactly this many single-candidate entries
        // and refuses one more (going through `encode_catalog_entries`,
        // which -- unlike `CatalogV1::encode` -- does not apply the
        // tighter admission ceiling, so this measures the wire format's
        // limit and nothing about this device's own policy).
        let at_ceiling = many_single_candidate_entries(MAX_ENCODABLE_CATALOG_ENTRIES);
        let encoded = encode_catalog_entries(&at_ceiling).unwrap();
        assert!(encoded.len() <= coding_adventures_canonical_cbor::MAX_ENCODED_SIZE);

        let one_more = many_single_candidate_entries(MAX_ENCODABLE_CATALOG_ENTRIES + 1);
        assert_eq!(
            encode_catalog_entries(&one_more),
            Err(ApplicationError::BoundExceeded),
        );
    }

    #[test]
    fn admission_refuses_growth_before_the_catalog_is_ever_unencodable() {
        // Before this fix, `validate_catalog` admitted up to a flat,
        // eyeballed 100,000 entries -- a number the encoder beneath it
        // could never actually reach. Admission now refuses the entry that
        // would make the catalog unencodable, before that catalog is ever
        // built, rather than discovering the failure on the next re-encode.
        let at_ceiling = many_single_candidate_entries(MAX_CATALOG_ENTRIES);
        let catalog = CatalogV1::new(at_ceiling).unwrap();
        let encoded = catalog.encode().unwrap();
        assert_eq!(CatalogV1::decode(&encoded).unwrap(), catalog);

        let one_more = many_single_candidate_entries(MAX_CATALOG_ENTRIES + 1);
        assert_eq!(
            CatalogV1::new(one_more),
            Err(ApplicationError::BoundExceeded),
            "admission must reject the (MAX_CATALOG_ENTRIES + 1)-th item \
             itself, not merely fail later on encode",
        );

        // A catalog that does fit still encodes and round trips, so the
        // check has not simply closed the door on every large vault.
        let small = many_single_candidate_entries(1_000);
        let catalog = CatalogV1::new(small).unwrap();
        let encoded = catalog.encode().unwrap();
        assert_eq!(CatalogV1::decode(&encoded).unwrap(), catalog);
    }

    #[test]
    fn decode_stays_open_to_a_legacy_or_peer_catalog_admission_would_now_refuse() {
        // Between the two bounds -- admission's MAX_CATALOG_ENTRIES and the
        // proven MAX_ENCODABLE_CATALOG_ENTRIES ceiling -- is exactly the
        // band a fully honest encoder (this device before this fix shipped,
        // or any other device honouring the same wire format) could
        // legitimately have produced, but this device would no longer admit
        // building itself. Decode must still accept it: denying it would
        // turn "this device won't grow the catalog further" into "this
        // device can no longer open its own history," which is the failure
        // VLT-PM05 section 13.3 already ruled out once for individual
        // records.
        const BETWEEN: usize = MAX_CATALOG_ENTRIES + 500;
        const _: () = assert!(MAX_CATALOG_ENTRIES < MAX_ENCODABLE_CATALOG_ENTRIES);
        const _: () = assert!(BETWEEN < MAX_ENCODABLE_CATALOG_ENTRIES);
        let between = BETWEEN;

        let entries = many_single_candidate_entries(between);
        // This device would no longer admit constructing it fresh...
        assert_eq!(
            CatalogV1::new(entries.clone()),
            Err(ApplicationError::BoundExceeded),
        );
        // ...but it decodes and opens, because a past version of this
        // device (or any honest peer) could have produced these exact
        // bytes through this exact encoder.
        let encoded = encode_catalog_entries(&entries).unwrap();
        let decoded = CatalogV1::decode(&encoded).unwrap();
        assert_eq!(decoded.entries(), &entries);
    }

    #[test]
    fn mutation_of_an_above_admission_catalog_succeeds_when_it_does_not_grow() {
        // A first version of this fix applied the tight MAX_CATALOG_ENTRIES
        // ceiling unconditionally to every catalog rebuild, admission and
        // mutation alike. That reopened the exact bug this fix exists to
        // close, just at a narrower band: a catalog synced from a peer, or
        // grown under this device's own pre-fix admission policy, with an
        // entry count anywhere in (MAX_CATALOG_ENTRIES,
        // MAX_ENCODABLE_CATALOG_ENTRIES] would decode and open (decode uses
        // the looser ceiling) but then fail *every* subsequent mutation --
        // including delete -- because rebuilding that same, unchanged entry
        // count through the tight bound always failed, regardless of
        // whether the mutation added anything. `new_for_mutation` is the
        // fix: it only applies the tight ceiling when entry count actually
        // grows past what it already was.
        const ABOVE_ADMISSION: usize = MAX_CATALOG_ENTRIES + 500;
        const _: () = assert!(ABOVE_ADMISSION < MAX_ENCODABLE_CATALOG_ENTRIES);

        let entries = many_single_candidate_entries(ABOVE_ADMISSION);

        // Same entry count as "before" (an edit, delete, or restore of an
        // item that already had a catalog entry): allowed, because nothing
        // grew, even though the count is already past MAX_CATALOG_ENTRIES.
        let catalog = CatalogV1::new_for_mutation(entries.clone(), ABOVE_ADMISSION).unwrap();
        // And it must actually re-encode -- this is the failure mode the
        // bug exhibited: admission accepting a catalog that then can't be
        // turned back into bytes.
        let encoded = catalog.encode().unwrap();
        assert_eq!(CatalogV1::decode(&encoded).unwrap(), catalog);

        // One MORE entry than "before" (a brand-new item): this is genuine
        // growth, so the tight admission ceiling still applies and refuses
        // it, exactly as it should.
        let mut grown = entries.clone();
        grown.insert(ItemId::new([0xff; 16]), vec![RevisionId::new([1; 32])]);
        assert_eq!(
            CatalogV1::new_for_mutation(grown, ABOVE_ADMISSION),
            Err(ApplicationError::BoundExceeded),
        );

        // Fewer entries than "before" is never growth either, so it is
        // allowed on the same terms as the unchanged case, all the way up
        // to the proven ceiling.
        let mut shrunk = entries;
        shrunk.remove(shrunk.keys().next().copied().as_ref().unwrap());
        let catalog = CatalogV1::new_for_mutation(shrunk, ABOVE_ADMISSION).unwrap();
        assert!(catalog.encode().is_ok());
    }

    #[test]
    fn decode_rejects_a_catalog_no_honest_encoder_could_have_produced() {
        // Past MAX_ENCODABLE_CATALOG_ENTRIES, no device honouring
        // MAX_ENCODED_SIZE -- old or new -- could ever have written these
        // bytes, so rejecting them costs no legitimate peer or legacy vault
        // anything. This is the decode-time half of the fix: a peer whose
        // own framing budget is looser than this wire format's cannot hand
        // this device a catalog it can decode but never again re-encode.
        let too_many = MAX_ENCODABLE_CATALOG_ENTRIES + 1;
        let peer_bytes = raw_catalog_bytes(too_many);
        assert!(peer_bytes.len() <= MAX_PLAINTEXT_BYTES);
        assert_eq!(
            CatalogV1::decode(&peer_bytes),
            Err(ApplicationError::BoundExceeded),
        );

        // The control: one entry under the same ceiling, built the same
        // hand-assembled way, must still decode -- so the rejection above
        // is about count, not about how the bytes were authored.
        let at_ceiling = raw_catalog_bytes(MAX_ENCODABLE_CATALOG_ENTRIES);
        assert!(CatalogV1::decode(&at_ceiling).is_ok());
    }

    #[test]
    fn oversized_record_is_reported_rather_than_aborting_the_process() {
        // This crate's plaintext gate is 16 MiB; canonical-CBOR's encoded
        // ceiling is 1 MiB. A 2 MiB password sits between them: legal to
        // hold, legal to decode, and refused by the encoder. It used to
        // abort the process instead.
        const _: () = assert!(
            HUGE_PASSWORD_BYTES < MAX_PLAINTEXT_BYTES,
            "the poisoned record must sit inside our own plaintext gate, \
             so that only the codec's tighter ceiling rejects it"
        );
        let record = AnyRecord::Login(Login {
            title: String::new(),
            username: String::new(),
            password: "a".repeat(HUGE_PASSWORD_BYTES),
            urls: Vec::new(),
            notes: None,
        });
        assert_eq!(
            encode_any_record(&record),
            Err(ApplicationError::BoundExceeded)
        );

        // And through the revision encode that wraps it.
        assert_eq!(
            encode_item_revision(
                candidate_with_password_len(HUGE_PASSWORD_BYTES).causal_parents(),
                candidate_with_password_len(HUGE_PASSWORD_BYTES).state(),
            ),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn deleting_an_oversized_item_stays_possible() {
        // The escape hatch. An oversized record that a peer already
        // synced can never be re-encoded, so `item edit` and the merges
        // are permanently closed for it. Deletion is what keeps that from
        // being a trap, and it works because a tombstone revision carries
        // only the item id and a timestamp -- the record is not in it.
        //
        // This is what stops one poisoned record from being unrecoverable,
        // and it is also how an operator clears the way for a blocked
        // export, so it is pinned rather than left implicit.
        let live = candidate_with_password_len(HUGE_PASSWORD_BYTES);
        assert_eq!(
            encode_item_revision(live.causal_parents(), live.state()),
            Err(ApplicationError::BoundExceeded)
        );

        let tombstone = ItemCandidate::new(
            RevisionId::new([0x52; 32]),
            [RevisionId::new([0x51; 32])],
            ItemState::Tombstone(Tombstone {
                // The very item whose live revision was refused above.
                item_id: ItemId::new([0x21; 16]),
                deleted_at_ms: 44,
            }),
        )
        .unwrap();
        let encoded = encode_item_revision(tombstone.causal_parents(), tombstone.state())
            .expect("deleting an oversized item must remain possible");
        assert_eq!(
            decode_item_revision(tombstone.revision_id(), &encoded).unwrap(),
            tombstone
        );
    }

    #[test]
    fn encode_errors_keep_size_and_integrity_faults_distinct() {
        // Only the two ceilings mean "too big to represent". Every other
        // CBOR encode failure is a canonicality fault and must not be
        // laundered into a benign size error -- DuplicateMapKey above all,
        // since it means the value would encode to bytes the strict
        // decoder rejects as ambiguous.
        assert_eq!(
            map_encode_error(CborError::EncodeTooLarge),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_encode_error(CborError::EncodeTooDeep),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_encode_error(CborError::DuplicateMapKey),
            ApplicationError::IntegrityFailure
        );
        assert_eq!(
            map_encode_error(CborError::NonCanonicalMapOrder),
            ApplicationError::IntegrityFailure
        );

        // And the same split one layer up, through the record codec's
        // own error type.
        assert_eq!(
            map_record_encode_error(VaultRecordError::Cbor(CborError::EncodeTooLarge)),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_record_encode_error(VaultRecordError::Cbor(CborError::DuplicateMapKey)),
            ApplicationError::IntegrityFailure
        );
        assert_eq!(
            map_record_encode_error(VaultRecordError::NotARecord),
            ApplicationError::IntegrityFailure
        );
    }

    #[test]
    fn encode_item_revision_pins_the_exact_encoded_size_boundary() {
        use coding_adventures_canonical_cbor::MAX_ENCODED_SIZE;

        // The boundary is derived rather than guessed so it stays exact if
        // the revision shape or the ceiling ever moves. The probe length is
        // chosen inside the same 5-byte CBOR length-prefix bracket as
        // MAX_ENCODED_SIZE (65_536 ..= 4_294_967_295), so the measured
        // overhead is genuinely constant across the arithmetic below.
        const PROBE: usize = 65_536;
        let probe = candidate_with_password_len(PROBE);
        let probe_len = encode_item_revision(probe.causal_parents(), probe.state())
            .expect("the probe is far below the ceiling")
            .len();
        let overhead = probe_len - PROBE;

        // Exactly at the ceiling: accepted, and exactly that size.
        let largest = MAX_ENCODED_SIZE - overhead;
        let at_ceiling = candidate_with_password_len(largest);
        let encoded = encode_item_revision(at_ceiling.causal_parents(), at_ceiling.state())
            .expect("a revision encoding to exactly MAX_ENCODED_SIZE is legal");
        assert_eq!(encoded.len(), MAX_ENCODED_SIZE);
        // It really is a whole revision, not a truncated one.
        assert_eq!(
            decode_item_revision(at_ceiling.revision_id(), &encoded).unwrap(),
            at_ceiling
        );

        // One byte more: refused, not panicked.
        let over = candidate_with_password_len(largest + 1);
        assert_eq!(
            encode_item_revision(over.causal_parents(), over.state()),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn revision_framing_is_a_second_failure_point_beyond_the_record_encode() {
        use coding_adventures_canonical_cbor::MAX_ENCODED_SIZE;

        // The two encodes fail independently. This picks a password for
        // which the *record* still encodes -- proving the inner gate is
        // satisfied -- while the revision map wrapped around it (item id,
        // schema tag, timestamps, favourite register, three observed sets,
        // causal parents) pushes the total over the same ceiling.
        //
        // Choosing it: start from the largest revision that fits, then add
        // back more than the revision's own framing overhead so the record
        // is under the ceiling while the revision is over it.
        const PROBE: usize = 65_536;
        let probe = candidate_with_password_len(PROBE);
        let revision_overhead = encode_item_revision(probe.causal_parents(), probe.state())
            .unwrap()
            .len()
            - PROBE;
        let record_overhead = match probe.state() {
            ItemState::Live(document) => encode_any_record(document.payload()).unwrap().len(),
            _ => unreachable!(),
        } - PROBE;
        assert!(
            record_overhead < revision_overhead,
            "the revision must cost more framing than the bare record"
        );

        let password_len = MAX_ENCODED_SIZE - revision_overhead + 1;
        let candidate = candidate_with_password_len(password_len);
        let ItemState::Live(document) = candidate.state() else {
            unreachable!()
        };

        // Inner encode succeeds ...
        let record = encode_any_record(document.payload())
            .expect("the record itself is still inside the ceiling");
        assert!(record.len() <= MAX_ENCODED_SIZE);

        // ... and the outer one still has to fail closed.
        assert_eq!(
            encode_item_revision(candidate.causal_parents(), candidate.state()),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn item_revision_round_trip_retains_removed_observations() {
        let candidate = live_candidate();
        let encoded = encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap();
        let decoded = decode_item_revision(candidate.revision_id(), &encoded).unwrap();
        assert_eq!(decoded, candidate);
        let ItemState::Live(document) = decoded.state() else {
            panic!("expected live state")
        };
        assert_eq!(document.collection_ids().retained_value_count(), 1);
        assert_eq!(document.collection_ids().add_operation_count(), 2);
        assert_eq!(document.collection_ids().tombstone_count(), 1);
        assert_eq!(document.tags().len(), 0);
        assert_eq!(document.tags().retained_value_count(), 1);
        assert_eq!(document.tags().tombstone_count(), 1);
    }

    /// VLT-PM47 §4.7 and §9.5. The compatibility claim, stated as bytes.
    ///
    /// An item with no attachments must encode the same nine keys it encoded
    /// before attachments existed — so every revision written by every earlier
    /// version of this product still decodes, and every revision this product
    /// writes for an unattached item is unchanged. An item that *has* one
    /// carries a tenth key and nothing else moves.
    #[test]
    fn the_tenth_live_state_field_appears_exactly_when_an_item_has_attachments() {
        fn live_state_keys(encoded: &[u8]) -> Vec<u64> {
            let CborValue::Map(revision) = decode(encoded).unwrap() else {
                panic!("a revision is a map")
            };
            let state = revision
                .into_iter()
                .find_map(|(key, value)| (key == CborValue::Unsigned(5)).then_some(value))
                .expect("field 5 is the state");
            let CborValue::Map(entries) = state else {
                panic!("live state is a map")
            };
            entries
                .into_iter()
                .map(|(key, _)| match key {
                    CborValue::Unsigned(key) => key,
                    _ => panic!("keys are unsigned"),
                })
                .collect()
        }

        let unattached = ItemDocument::new(
            ItemId::new([0x21; 16]),
            ContentType::new(LOGIN_V1).unwrap(),
            10,
            20,
            LwwRegister::new(true, 15, op(5)),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: "Example".to_string(),
                username: "alice".to_string(),
                password: "correct horse".to_string(),
                urls: Vec::new(),
                notes: None,
            }),
            ObservedSet::new(),
            BTreeMap::new(),
        )
        .unwrap();
        let parents = BTreeSet::new();
        let without =
            encode_item_revision(&parents, &ItemState::Live(Box::new(unattached))).unwrap();
        assert_eq!(live_state_keys(&without), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let with =
            encode_item_revision(live_candidate().causal_parents(), live_candidate().state())
                .unwrap();
        assert_eq!(live_state_keys(&with), vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        // Both shapes decode, and neither admits an eleventh key, a duplicate,
        // or an empty tenth field: this is still a closed decoder with two
        // accepted key sets rather than an open one.
        assert!(decode_item_revision(RevisionId::new([1; 32]), &without).is_ok());
        assert!(decode_item_revision(RevisionId::new([1; 32]), &with).is_ok());
    }

    /// The decoder's own copies of key material are wiped on every exit, not
    /// only the successful one.
    ///
    /// A test cannot look at freed heap, so it asserts the mechanism instead:
    /// the field map handed back from a failed `value_fields`, and the one
    /// `AttachmentManifestV1::decode` leaves behind on each of its three
    /// pre-key checks, must contain no live copy of the key bytes.
    #[test]
    fn a_rejected_decode_wipes_the_key_material_it_had_already_taken() {
        const KEY: [u8; ATTACHMENT_DEK_BYTES] = [0xA7; ATTACHMENT_DEK_BYTES];

        fn holds_key(value: &CborValue) -> bool {
            match value {
                CborValue::Bytes(bytes) => bytes.as_slice() == KEY.as_slice(),
                CborValue::Array(values) => values.iter().any(holds_key),
                CborValue::Map(entries) => entries
                    .iter()
                    .any(|(key, value)| holds_key(key) || holds_key(value)),
                CborValue::Tag(_, inner) => holds_key(inner),
                _ => false,
            }
        }

        let manifest = |version: u64, kind: u64, id: CborValue| {
            encode(&CborValue::Map(vec![
                field(1, CborValue::Unsigned(version)),
                field(2, CborValue::Unsigned(kind)),
                field(3, id),
                field(4, bytes(&KEY)),
                field(5, CborValue::text("notes.txt")),
                field(6, CborValue::Unsigned(3)),
                field(7, bytes(&[4; 32])),
                field(8, CborValue::Array(vec![bytes(&[5; 32])])),
                field(9, CborValue::Unsigned(6)),
            ]))
        };

        // The three checks that run *before* the key is taken. Each one used
        // to drop the map, and the key with it, unwiped.
        let good_id = bytes(&[1; 16]);
        for (label, encoded) in [
            (
                "wrong version",
                manifest(2, ObjectKind::AttachmentManifest.code(), good_id.clone()),
            ),
            (
                "wrong kind",
                manifest(VERSION, ObjectKind::Catalog.code(), good_id.clone()),
            ),
            (
                "malformed attachment id",
                manifest(
                    VERSION,
                    ObjectKind::AttachmentManifest.code(),
                    bytes(&[1; 15]),
                ),
            ),
        ] {
            assert!(
                AttachmentManifestV1::decode(&encoded).is_err(),
                "{label} must be refused"
            );
            // Re-run the same split the decoder performs, and confirm the
            // wipe leaves nothing behind for the failing path to drop.
            let mut fields = closed_fields(&encoded, &[1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap();
            assert!(
                fields.values().any(holds_key),
                "{label}: the fixture must actually contain the key"
            );
            let outcome = AttachmentManifestV1::decode_fields(&mut fields);
            assert!(outcome.is_err(), "{label}");
            for value in fields.values_mut() {
                zeroize_cbor_secrets(value);
            }
            assert!(
                !fields.values().any(holds_key),
                "{label}: a refused decode left the key in the field map"
            );
        }

        // And `value_fields` itself, which fails before the decoder ever sees
        // a map: a duplicate key, with the key material already collected.
        let duplicate = CborValue::Map(vec![
            field(1, bytes(&KEY)),
            field(1, CborValue::Unsigned(2)),
        ]);
        assert_eq!(
            value_fields(duplicate, &[1, 2]).unwrap_err(),
            ApplicationError::IntegrityFailure
        );

        // The wiping helper reaches text as well as bytes, so a helper whose
        // guarantee depends on which variant a future field uses is not what
        // this crate has.
        let mut mixed = CborValue::Map(vec![
            field(1, bytes(&KEY)),
            field(2, CborValue::text("secret-name")),
            field(3, CborValue::Array(vec![bytes(&KEY)])),
        ]);
        zeroize_cbor_secrets(&mut mixed);
        assert!(!holds_key(&mixed));
        let CborValue::Map(entries) = &mixed else {
            panic!("still a map")
        };
        assert_eq!(entries[1].1, CborValue::Text(String::new()));
    }

    /// Every shape a peer could give the tenth field that the encoder cannot
    /// produce, refused — and none of them a panic.
    #[test]
    fn a_malformed_attachment_manifest_field_is_refused() {
        let attachment = |byte: u8| bytes(&[byte; 16]);
        let manifest = |byte: u8| bytes(&[byte; 32]);
        let entry = |id: u8, target: u8| {
            CborValue::Map(vec![field(1, attachment(id)), field(2, manifest(target))])
        };

        for (name, value) in [
            ("not an array", CborValue::Unsigned(1)),
            ("empty array", CborValue::Array(Vec::new())),
            (
                "descending ids",
                CborValue::Array(vec![entry(2, 1), entry(1, 1)]),
            ),
            (
                "duplicate ids",
                CborValue::Array(vec![entry(1, 1), entry(1, 1)]),
            ),
            (
                "short manifest id",
                CborValue::Array(vec![CborValue::Map(vec![
                    field(1, attachment(1)),
                    field(2, bytes(&[0; 31])),
                ])]),
            ),
            (
                "extra entry key",
                CborValue::Array(vec![CborValue::Map(vec![
                    field(1, attachment(1)),
                    field(2, manifest(1)),
                    field(3, CborValue::Unsigned(0)),
                ])]),
            ),
        ] {
            let error = decode_attachment_manifest_references(value)
                .expect_err(&format!("{name} must be refused"));
            assert!(
                matches!(
                    error,
                    ApplicationError::IntegrityFailure | ApplicationError::BoundExceeded
                ),
                "{name} produced {error:?}"
            );
        }

        // A list longer than any observed set can hold is a bound, not a
        // corruption, and it is refused before the map is built.
        let oversized = CborValue::Array(
            (0..=MAX_OBSERVED_VALUES)
                .map(|index| {
                    CborValue::Map(vec![
                        field(1, bytes(&(index as u128).to_be_bytes())),
                        field(2, manifest(1)),
                    ])
                })
                .collect(),
        );
        assert_eq!(
            decode_attachment_manifest_references(oversized).unwrap_err(),
            ApplicationError::BoundExceeded
        );
    }

    /// A manifest is an object a synchronising peer can author, so every one
    /// of its numbers is input. None of these may abort.
    #[test]
    fn a_malformed_attachment_manifest_object_is_refused() {
        let good = AttachmentManifestV1::new(
            AttachmentId::new([1; 16]),
            Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
            "notes.txt".to_string(),
            3,
            [4; 32],
            vec![ObjectId::new([5; 32])],
            6,
        )
        .unwrap();
        let encoded = good.encode().unwrap();
        let decoded = AttachmentManifestV1::decode(&encoded).unwrap();
        assert_eq!(decoded.attachment_id(), AttachmentId::new([1; 16]));
        assert_eq!(decoded.name(), "notes.txt");
        assert_eq!(decoded.total_plaintext_len(), 3);
        assert_eq!(decoded.content_sha256(), &[4; 32]);
        assert_eq!(decoded.chunks(), &[ObjectId::new([5; 32])]);
        assert_eq!(decoded.created_at_ms(), 6);
        assert!(format!("{decoded:?}").contains("chunk_count: 1"));
        assert!(!format!("{decoded:?}").contains("notes.txt"));

        // A declared length that does not fit the chunk list, in both
        // directions. One of the two numbers is a lie and the reassembler
        // should never have to decide which.
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "notes.txt".to_string(),
                ATTACHMENT_CHUNK_BYTES as u64 + 1,
                [4; 32],
                vec![ObjectId::new([5; 32])],
                6,
            )
            .unwrap_err(),
            ApplicationError::IntegrityFailure
        );
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "notes.txt".to_string(),
                3,
                [4; 32],
                Vec::new(),
                6,
            )
            .unwrap_err(),
            ApplicationError::IntegrityFailure
        );
        // Duplicate chunk references cannot occur: every chunk carries a
        // distinct nonce and a distinct random object key.
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "notes.txt".to_string(),
                ATTACHMENT_CHUNK_BYTES as u64 + 1,
                [4; 32],
                vec![ObjectId::new([5; 32]), ObjectId::new([5; 32])],
                6,
            )
            .unwrap_err(),
            ApplicationError::IntegrityFailure
        );
        // Past the ceiling in either dimension is a bound.
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "notes.txt".to_string(),
                MAX_ATTACHMENT_BYTES as u64 + 1,
                [4; 32],
                vec![ObjectId::new([5; 32])],
                6,
            )
            .unwrap_err(),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "notes.txt".to_string(),
                3,
                [4; 32],
                (0..=MAX_ATTACHMENT_CHUNKS)
                    .map(|index| ObjectId::new([index as u8; 32]))
                    .collect(),
                6,
            )
            .unwrap_err(),
            ApplicationError::BoundExceeded
        );
        // An invalid name never becomes a stored name.
        assert_eq!(
            AttachmentManifestV1::new(
                AttachmentId::new([1; 16]),
                Zeroizing::new([2; ATTACHMENT_DEK_BYTES]),
                "../escape".to_string(),
                3,
                [4; 32],
                vec![ObjectId::new([5; 32])],
                6,
            )
            .unwrap_err(),
            ApplicationError::InvalidInput
        );

        // A frame of another kind at the manifest's address does not decode
        // into a manifest, because the kind is bound into the payload.
        let mut wrong_kind = encoded.clone();
        let catalog = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(ObjectKind::Catalog.code())),
            field(3, CborValue::Array(Vec::new())),
        ]));
        wrong_kind.clone_from(&catalog);
        assert_eq!(
            AttachmentManifestV1::decode(&wrong_kind).unwrap_err(),
            ApplicationError::IntegrityFailure
        );
    }

    /// The chunk object is the value whose encoded size the whole design turns
    /// on, and it is also peer-authored.
    #[test]
    fn an_attachment_chunk_object_round_trips_and_refuses_oversize() {
        let chunk = EncryptedChunk {
            blob_id: BlobId([7; 16]),
            index: 3,
            is_final: true,
            ciphertext: vec![9; ATTACHMENT_CHUNK_BYTES],
            tag: [1; 16],
        };
        let encoded = encode_attachment_chunk(&chunk).unwrap();
        // The number the design turns on: a full chunk object sits far under
        // canonical-CBOR's ceiling, and cannot grow with the attachment.
        assert!(encoded.len() < ATTACHMENT_CHUNK_BYTES + 128);
        assert!(encoded.len() * 15 <= coding_adventures_canonical_cbor::MAX_ENCODED_SIZE);
        assert_eq!(decode_attachment_chunk(&encoded).unwrap(), chunk);

        let oversized = EncryptedChunk {
            ciphertext: vec![9; ATTACHMENT_CHUNK_BYTES + 1],
            ..chunk.clone()
        };
        assert_eq!(
            encode_attachment_chunk(&oversized).unwrap_err(),
            ApplicationError::BoundExceeded
        );
        let far_index = EncryptedChunk {
            index: MAX_ATTACHMENT_CHUNKS as u32,
            ..chunk.clone()
        };
        assert_eq!(
            encode_attachment_chunk(&far_index).unwrap_err(),
            ApplicationError::BoundExceeded
        );

        // A peer is not bound by our encoder, so the decoder repeats both
        // bounds rather than trusting it.
        let peer_oversized = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(ObjectKind::AttachmentChunk.code())),
            field(3, bytes(&[7; 16])),
            field(4, CborValue::Unsigned(3)),
            field(5, CborValue::Bool(true)),
            field(6, bytes(&vec![9; ATTACHMENT_CHUNK_BYTES + 1])),
            field(7, bytes(&[1; 16])),
        ]));
        assert_eq!(
            decode_attachment_chunk(&peer_oversized).unwrap_err(),
            ApplicationError::BoundExceeded
        );
        let peer_far_index = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(ObjectKind::AttachmentChunk.code())),
            field(3, bytes(&[7; 16])),
            field(4, CborValue::Unsigned(u64::MAX)),
            field(5, CborValue::Bool(true)),
            field(6, bytes(&[9; 4])),
            field(7, bytes(&[1; 16])),
        ]));
        assert_eq!(
            decode_attachment_chunk(&peer_far_index).unwrap_err(),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            decode_attachment_chunk(&encode(&CborValue::Unsigned(1))).unwrap_err(),
            ApplicationError::IntegrityFailure
        );
    }

    #[test]
    fn tombstone_revision_round_trips_and_rejects_bad_headers() {
        let candidate = ItemCandidate::new(
            RevisionId::new([9; 32]),
            [],
            ItemState::Tombstone(Tombstone {
                item_id: ItemId::new([8; 16]),
                deleted_at_ms: 44,
            }),
        )
        .unwrap();
        let encoded = encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap();
        assert_eq!(
            hex(&encoded),
            "a5010102010380040205a201500808080808080808080808080808080802182c"
        );
        assert_eq!(
            decode_item_revision(candidate.revision_id(), &encoded).unwrap(),
            candidate
        );

        let wrong_kind = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(1)),
            field(2, CborValue::Unsigned(ObjectKind::Catalog.code())),
            field(3, CborValue::Array(Vec::new())),
            field(4, CborValue::Unsigned(TOMBSTONE_STATE)),
            field(
                5,
                CborValue::Map(vec![
                    field(1, bytes(&[8; 16])),
                    field(2, CborValue::Unsigned(44)),
                ]),
            ),
        ]));
        assert_eq!(
            decode_item_revision(RevisionId::new([9; 32]), &wrong_kind),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn catalog_round_trip_is_sorted_bounded_and_redacted() {
        let mut entries = BTreeMap::new();
        entries.insert(
            ItemId::new([1; 16]),
            vec![RevisionId::new([2; 32]), RevisionId::new([3; 32])],
        );
        entries.insert(ItemId::new([4; 16]), vec![RevisionId::new([5; 32])]);
        let catalog = CatalogV1::new(entries).unwrap();
        let encoded = catalog.encode().unwrap();
        assert_eq!(CatalogV1::decode(&encoded).unwrap(), catalog);
        assert_eq!(format!("{catalog:?}"), "CatalogV1 { entry_count: 2 }");
        assert!(CatalogV1::empty().entries().is_empty());
        assert_eq!(hex(&CatalogV1::empty().encode().unwrap()), "a3010102020380");
    }

    #[test]
    fn signed_object_wrappers_are_exact_closed_and_kind_bound() {
        let certificate = DeviceCertificateV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([2; 16]),
            signing_public_key: PublicKey::new([3; 32]),
            wrapping_public_key: PublicKey::new([4; 32]),
            created_at_ms: 5,
            capabilities: Vec::new(),
            signature: Signature::new([6; 64]),
        };
        let encoded_certificate = encode_device_certificate(&certificate).unwrap();
        assert_eq!(
            hex(&encoded_certificate),
            "a3010102030358b4a80101025001010101010101010101010101010101035002020202020202020202020202020202045820030303030303030303030303030303030303030303030303030303030303030305582004040404040404040404040404040404040404040404040404040404040404040605078008584006060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606"
        );
        assert_eq!(
            decode_device_certificate(&encoded_certificate).unwrap(),
            certificate
        );

        let commit = CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([2; 16]),
            device_counter: 1,
            parents: Vec::new(),
            catalog_root: ObjectId::new([3; 32]),
            added_objects: vec![ObjectId::new([3; 32])],
            tombstone_root: None,
            wall_time_ms: 5,
            device_certificate: ObjectId::new([4; 32]),
            signature: Signature::new([6; 64]),
        };
        let encoded_commit = encode_signed_commit(&commit).unwrap();
        assert_eq!(
            hex(&encoded_commit),
            "a3010102040358dcab010102500101010101010101010101010101010103500202020202020202020202020202020204010580065820030303030303030303030303030303030303030303030303030303030303030307815820030303030303030303030303030303030303030303030303030303030303030308f609050a582004040404040404040404040404040404040404040404040404040404040404040b584006060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606060606"
        );
        assert_eq!(decode_signed_commit(&encoded_commit).unwrap(), commit);

        let audit_seed = [7; 32];
        let (audit_public_key, _) = generate_keypair(&audit_seed);
        let audit_event = AuditEventV1::new(
            VaultId::new([1; 16]),
            DeviceId::new([2; 16]),
            2,
            OperationId::new([8; 32]),
            AuditActionV1::AuditEpochStart,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            None,
            None,
            vec![ObjectId::new([9; 32])],
            10,
        )
        .unwrap()
        .sign(&audit_seed)
        .unwrap();
        let encoded_audit_event = encode_signed_audit_event(&audit_event).unwrap();
        let decoded_audit_event = decode_signed_audit_event(&encoded_audit_event).unwrap();
        assert_eq!(decoded_audit_event, audit_event);
        decoded_audit_event.verify(&audit_public_key).unwrap();

        assert_eq!(
            decode_signed_commit(&encoded_certificate),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            decode_signed_audit_event(&encoded_commit),
            Err(ApplicationError::IntegrityFailure)
        );
        let malformed_nested = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(ObjectKind::DeviceCertificate.code())),
            field(3, CborValue::Bytes(vec![0xff])),
        ]));
        assert_eq!(
            decode_device_certificate(&malformed_nested),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn catalog_rejects_empty_unsorted_duplicate_and_wrong_kind_candidates() {
        let mut empty = BTreeMap::new();
        empty.insert(ItemId::new([1; 16]), Vec::new());
        assert_eq!(CatalogV1::new(empty), Err(ApplicationError::InvalidInput));

        let mut unsorted = BTreeMap::new();
        unsorted.insert(
            ItemId::new([1; 16]),
            vec![RevisionId::new([3; 32]), RevisionId::new([2; 32])],
        );
        assert_eq!(
            CatalogV1::new(unsorted),
            Err(ApplicationError::InvalidInput)
        );

        let wrong_kind = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(1)),
            field(2, CborValue::Unsigned(ObjectKind::ItemRevision.code())),
            field(3, CborValue::Array(Vec::new())),
        ]));
        assert_eq!(
            CatalogV1::decode(&wrong_kind),
            Err(ApplicationError::IntegrityFailure)
        );

        let empty_candidate = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(1)),
            field(2, CborValue::Unsigned(ObjectKind::Catalog.code())),
            field(
                3,
                CborValue::Array(vec![CborValue::Map(vec![
                    field(1, bytes(&[1; 16])),
                    field(2, CborValue::Array(Vec::new())),
                ])]),
            ),
        ]));
        assert_eq!(
            CatalogV1::decode(&empty_candidate),
            Err(ApplicationError::IntegrityFailure)
        );

        let unsorted_candidates = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(1)),
            field(2, CborValue::Unsigned(ObjectKind::Catalog.code())),
            field(
                3,
                CborValue::Array(vec![CborValue::Map(vec![
                    field(1, bytes(&[1; 16])),
                    field(2, CborValue::Array(vec![bytes(&[3; 32]), bytes(&[2; 32])])),
                ])]),
            ),
        ]));
        assert_eq!(
            CatalogV1::decode(&unsorted_candidates),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn item_revision_rejects_unsorted_parents_and_unknown_state() {
        let tombstone = CborValue::Map(vec![
            field(1, bytes(&[8; 16])),
            field(2, CborValue::Unsigned(44)),
        ]);
        let invalid = |parents: Vec<CborValue>, state| {
            encode(&CborValue::Map(vec![
                field(1, CborValue::Unsigned(1)),
                field(2, CborValue::Unsigned(ObjectKind::ItemRevision.code())),
                field(3, CborValue::Array(parents)),
                field(4, CborValue::Unsigned(state)),
                field(5, tombstone.clone()),
            ]))
        };
        assert_eq!(
            decode_item_revision(
                RevisionId::new([9; 32]),
                &invalid(vec![bytes(&[2; 32]), bytes(&[1; 32])], TOMBSTONE_STATE)
            ),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            decode_item_revision(RevisionId::new([9; 32]), &invalid(Vec::new(), 99)),
            Err(ApplicationError::Unsupported)
        );
    }

    #[test]
    fn observed_set_decoder_rejects_malformed_or_lossy_state() {
        assert_eq!(
            decode_observed::<String>(CborValue::Bool(false)),
            Err(ApplicationError::IntegrityFailure)
        );
        let entry = |value: CborValue, adds: CborValue, removals: CborValue| {
            CborValue::Map(vec![field(1, value), field(2, adds), field(3, removals)])
        };
        assert_eq!(
            decode_observed::<String>(CborValue::Array(vec![entry(
                CborValue::Unsigned(1),
                CborValue::Array(vec![bytes(&[1; 32])]),
                CborValue::Array(Vec::new()),
            )])),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            decode_observed::<String>(CborValue::Array(vec![entry(
                CborValue::text("tag"),
                CborValue::Array(Vec::new()),
                CborValue::Array(Vec::new()),
            )])),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            decode_operations(CborValue::Bool(false)),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            decode_operations(CborValue::Array(vec![bytes(&[2; 32]), bytes(&[1; 32])])),
            Err(ApplicationError::IntegrityFailure)
        );
        let duplicate_values = CborValue::Array(vec![
            entry(
                CborValue::text("tag"),
                CborValue::Array(vec![bytes(&[1; 32])]),
                CborValue::Array(Vec::new()),
            ),
            entry(
                CborValue::text("tag"),
                CborValue::Array(vec![bytes(&[2; 32])]),
                CborValue::Array(Vec::new()),
            ),
        ]);
        assert_eq!(
            decode_observed::<String>(duplicate_values),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn primitive_parser_helpers_fail_closed() {
        assert_eq!(
            value_fields(CborValue::Bool(false), &[]),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            value_fields(
                CborValue::Map(vec![(CborValue::text("bad"), CborValue::Null)]),
                &[1],
            ),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            value_fields(
                CborValue::Map(vec![field(1, CborValue::Null), field(1, CborValue::Null),]),
                &[1, 1],
            ),
            Err(ApplicationError::IntegrityFailure)
        );

        let mut fields = BTreeMap::new();
        fields.insert(1, CborValue::Bool(false));
        assert_eq!(
            take_uint(&mut fields, 1),
            Err(ApplicationError::IntegrityFailure)
        );
        let mut fields = BTreeMap::new();
        fields.insert(1, CborValue::Unsigned(1));
        assert_eq!(
            take_bool(&mut fields, 1),
            Err(ApplicationError::IntegrityFailure)
        );
        let mut fields = BTreeMap::new();
        fields.insert(1, CborValue::Bytes(Vec::new()));
        assert_eq!(
            take_text(&mut fields, 1),
            Err(ApplicationError::IntegrityFailure)
        );
        let mut fields = BTreeMap::new();
        fields.insert(1, CborValue::Text(String::new()));
        assert_eq!(
            take_bytes(&mut fields, 1),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            fixed_value::<16>(CborValue::Bool(false)),
            Err(ApplicationError::IntegrityFailure)
        );
        let mut fields = BTreeMap::new();
        fields.insert(1, CborValue::Bool(false));
        assert_eq!(
            take_array(&mut fields, 1),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(check_version(2), Err(ApplicationError::Unsupported));
        assert_eq!(
            map_domain(coding_adventures_vault_pm_domain::DomainError::BoundExceeded),
            ApplicationError::BoundExceeded
        );
        assert_eq!(
            map_domain(coding_adventures_vault_pm_domain::DomainError::InvalidTag),
            ApplicationError::IntegrityFailure
        );
        assert_eq!(
            CatalogV1::decode(&vec![0; MAX_PLAINTEXT_BYTES + 1]),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn opaque_record_encoding_is_exact_and_rejects_bad_payload() {
        let record = AnyRecord::Opaque {
            content_type: "example/custom/v1".to_string(),
            payload_bytes: encode(&CborValue::Map(Vec::new())),
        };
        assert!(encode_any_record(&record).is_ok());
        let invalid = AnyRecord::Opaque {
            content_type: "example/custom/v1".to_string(),
            payload_bytes: vec![0xff],
        };
        assert_eq!(
            encode_any_record(&invalid),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn an_oversized_opaque_record_is_a_bound_not_an_integrity_fault() {
        use coding_adventures_canonical_cbor::MAX_ENCODED_SIZE;

        // Newly reachable: such a record could not be materialised until
        // `decode_record`'s opaque arm stopped re-encoding, so this arm
        // could not previously see a size failure from stored bytes. It
        // must not be reported in the vocabulary of corruption -- the
        // remedy is to delete one large item, which is the escape hatch of
        // VLT-PM05 13.3, not to rebuild a store that is perfectly intact.
        let mut payload = vec![0x5a_u8];
        payload.extend_from_slice(&(MAX_ENCODED_SIZE as u32).to_be_bytes());
        payload.extend(std::iter::repeat_n(0x5a_u8, MAX_ENCODED_SIZE));
        let oversized = AnyRecord::Opaque {
            content_type: "example/custom/v1".to_string(),
            payload_bytes: payload,
        };
        assert_eq!(
            encode_any_record(&oversized),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn application_errors_are_closed_and_payload_free() {
        for (error, label) in [
            (ApplicationError::NotInitialized, "NotInitialized"),
            (ApplicationError::AlreadyInitialized, "AlreadyInitialized"),
            (ApplicationError::Locked, "Locked"),
            (
                ApplicationError::AuthenticationFailed,
                "AuthenticationFailed",
            ),
            (ApplicationError::InvalidInput, "InvalidInput"),
            (ApplicationError::NotFound, "NotFound"),
            (ApplicationError::BoundExceeded, "BoundExceeded"),
            (ApplicationError::ConcurrentHost, "ConcurrentHost"),
            (ApplicationError::StorageUnavailable, "StorageUnavailable"),
            (ApplicationError::IntegrityFailure, "IntegrityFailure"),
            (ApplicationError::Unsupported, "Unsupported"),
            (ApplicationError::ConflictRequired, "ConflictRequired"),
            (ApplicationError::InternalInvariant, "InternalInvariant"),
        ] {
            assert_eq!(format!("{error:?}"), label);
            assert_eq!(format!("{error}"), format!("vault-pm-application: {label}"));
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
