use crate::codec::{MAX_CANDIDATES_PER_ITEM, MAX_CATALOG_ENTRIES};
use crate::initialize::{verify_active_bootstrap, verify_signed_bootstrap};
use crate::{decode_item_revision, encode_item_revision, ApplicationError};
use coding_adventures_argon2id::{argon2id, Options as Argon2idOptions};
use coding_adventures_canonical_cbor::{decode, encode, try_encode, CborValue};
use coding_adventures_chacha20_poly1305::{
    xchacha20_poly1305_aead_decrypt, xchacha20_poly1305_aead_encrypt,
};
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_domain::{ItemCandidate, ItemId, ItemState, RevisionId};
use coding_adventures_vault_pm_format::{Argon2idParametersV1, VaultId, CRYPTO_SUITE_V1};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const VERSION: u64 = 1;
const PASSPHRASE_PROTECTION: u64 = 1;
const KDF_SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 24;
const SNAPSHOT_HASH_DOMAIN: &[u8] = b"VPM-PORTABLE-SNAPSHOT-v1";
const EXPORT_AAD_DOMAIN: &[u8] = b"VPM-PORTABLE-EXPORT-AAD-v1";
const CANDIDATE_ESTIMATE_OVERHEAD: usize = 128;
const SNAPSHOT_ESTIMATE_OVERHEAD: usize = 1_024;
const ARTIFACT_OVERHEAD: usize = 4_096;

/// Exact host-supplied CSPRNG bytes consumed by one passphrase export.
pub const PORTABLE_EXPORT_RANDOM_BYTES: usize = KDF_SALT_BYTES + NONCE_BYTES;
/// Maximum separately collected export-passphrase bytes accepted by V1.
pub const MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES: usize = 1_024;
/// Maximum canonical plaintext snapshot size accepted by V1.
pub const MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES: usize = 512 * 1024 * 1024;
/// Maximum encrypted artifact bytes accepted by the V1 opener.
pub const MAX_PORTABLE_EXPORT_ARTIFACT_BYTES: usize =
    MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES + ARTIFACT_OVERHEAD;

/// Bounded caller-calibrated Argon2id policy for a portable export.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PortableExportPolicyV1 {
    memory_kib: u32,
    iterations: u32,
    lanes: u8,
}

impl PortableExportPolicyV1 {
    /// Validate and retain one V1 export-passphrase policy.
    pub fn new(memory_kib: u32, iterations: u32, lanes: u8) -> Result<Self, ApplicationError> {
        Argon2idParametersV1 {
            memory_kib,
            iterations,
            lanes,
            salt: [0; KDF_SALT_BYTES],
        }
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
        Ok(Self {
            memory_kib,
            iterations,
            lanes,
        })
    }

    /// Return the Argon2id memory cost in KiB.
    pub const fn memory_kib(&self) -> u32 {
        self.memory_kib
    }

    /// Return the Argon2id iteration count.
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Return the Argon2id lane count.
    pub const fn lanes(&self) -> u8 {
        self.lanes
    }
}

impl Debug for PortableExportPolicyV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortableExportPolicyV1")
            .field("memory_kib", &self.memory_kib)
            .field("iterations", &self.iterations)
            .field("lanes", &self.lanes)
            .finish()
    }
}

/// Host-approved resource ceiling for opening an untrusted portable artifact.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PortableOpenPolicyV1 {
    max_memory_kib: u32,
    max_iterations: u32,
    max_lanes: u8,
}

impl PortableOpenPolicyV1 {
    /// Validate one maximum Argon2id resource policy for artifact opening.
    pub fn new(
        max_memory_kib: u32,
        max_iterations: u32,
        max_lanes: u8,
    ) -> Result<Self, ApplicationError> {
        Argon2idParametersV1 {
            memory_kib: max_memory_kib,
            iterations: max_iterations,
            lanes: max_lanes,
            salt: [0; KDF_SALT_BYTES],
        }
        .validate()
        .map_err(|_| ApplicationError::InvalidInput)?;
        Ok(Self {
            max_memory_kib,
            max_iterations,
            max_lanes,
        })
    }

    fn allows(&self, kdf: &Argon2idParametersV1) -> bool {
        kdf.memory_kib <= self.max_memory_kib
            && kdf.iterations <= self.max_iterations
            && kdf.lanes <= self.max_lanes
    }
}

impl Debug for PortableOpenPolicyV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortableOpenPolicyV1")
            .field("max_memory_kib", &self.max_memory_kib)
            .field("max_iterations", &self.max_iterations)
            .field("max_lanes", &self.max_lanes)
            .finish()
    }
}

/// One owned wipe-on-drop CSPRNG block for export salt and nonce.
pub struct PortableExportRandomnessV1 {
    bytes: [u8; PORTABLE_EXPORT_RANDOM_BYTES],
}

impl PortableExportRandomnessV1 {
    /// Take one exact block filled by the host cryptographic entropy source.
    pub const fn new(bytes: [u8; PORTABLE_EXPORT_RANDOM_BYTES]) -> Self {
        Self { bytes }
    }

    fn salt(&self) -> [u8; KDF_SALT_BYTES] {
        self.bytes[..KDF_SALT_BYTES]
            .try_into()
            .expect("portable export salt partition is constant")
    }

    fn nonce(&self) -> [u8; NONCE_BYTES] {
        self.bytes[KDF_SALT_BYTES..]
            .try_into()
            .expect("portable export nonce partition is constant")
    }
}

impl Zeroize for PortableExportRandomnessV1 {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
    }
}

impl Drop for PortableExportRandomnessV1 {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl Debug for PortableExportRandomnessV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableExportRandomnessV1(<redacted>)")
    }
}

/// One canonical authenticated encrypted portable export artifact.
///
/// The bytes contain no plaintext vault record, owner-private state, local
/// pins, provider credential, or search projection. The host may write them to
/// an explicitly selected destination without interpreting their contents.
pub struct PortableExportArtifactV1 {
    bytes: Vec<u8>,
}

impl PortableExportArtifactV1 {
    /// Borrow the exact canonical encrypted artifact bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consume the wrapper and return the exact bytes for host persistence.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Debug for PortableExportArtifactV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("PortableExportArtifactV1(<encrypted>)")
    }
}

/// Selects how [`export_portable_with_passphrase`] treats a current candidate
/// it cannot re-encode (VLT-PM05 §13.9).
///
/// `Strict` is the original, still-default behavior every existing caller
/// gets unmodified: the first candidate that cannot be re-encoded (an
/// oversized first-party, opaque, or quarantined record — `BoundExceeded`,
/// VLT-PM05 §13.1/§13.3/§13.5) fails the whole export and leaves the vault
/// untouched, exactly as before this type existed. `BestEffort` is the new,
/// explicit, opt-in ceremony: an item whose current candidate set includes
/// one such record is left out of the snapshot in its entirety — every
/// current candidate of that item, never a subset (see
/// [`export_portable_with_passphrase`]'s own doc comment for why a subset
/// would silently misrepresent an unresolved conflict) — and its identity is
/// recorded in the snapshot's own authenticated `excluded_item_ids` field
/// instead. Every other item still exports normally. Neither variant ever
/// mutates the vault; both are read-only, exactly like the export this type
/// governs always was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableExportCompletenessV1 {
    /// Fail the whole export on the first unencodable candidate (default).
    Strict,
    /// Exclude the offending item and continue exporting everything else.
    BestEffort,
}

/// One completed [`PortableExportCompletenessV1::BestEffort`] export: the
/// encrypted artifact plus the items (if any) it excluded.
///
/// Every `ItemId` this exposes is already visible to the operator through
/// this same vault's own `item list` — nothing here discloses anything a
/// caller with an open session could not already see locally. A `Strict`
/// export never populates `excluded_item_ids`; it fails outright instead of
/// reaching this type, which is why the plain [`PortableExportArtifactV1`]
/// remains the `Strict` caller's return type unchanged.
pub struct PortableExportOutcomeV1 {
    artifact: PortableExportArtifactV1,
    excluded_item_ids: Vec<ItemId>,
}

impl PortableExportOutcomeV1 {
    /// Consume the outcome and return only the encrypted artifact.
    pub fn into_artifact(self) -> PortableExportArtifactV1 {
        self.artifact
    }

    /// Borrow the encrypted artifact without consuming the outcome.
    pub fn artifact(&self) -> &PortableExportArtifactV1 {
        &self.artifact
    }

    /// Borrow the identities of every item this export excluded.
    ///
    /// Empty unless the export ran under
    /// [`PortableExportCompletenessV1::BestEffort`] and at least one current
    /// candidate could not be re-encoded.
    pub fn excluded_item_ids(&self) -> &[ItemId] {
        &self.excluded_item_ids
    }
}

impl Debug for PortableExportOutcomeV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortableExportOutcomeV1")
            .field("artifact", &self.artifact)
            .field("excluded_item_count", &self.excluded_item_ids.len())
            .finish()
    }
}

/// Authenticated secret-bearing snapshot retained only inside the application.
///
/// The host can inspect aggregate counts before choosing whether to continue a
/// later import. Source identities, bootstrap bytes, candidate metadata, and
/// decrypted documents have no public accessor and diagnostics are redacted.
pub struct OpenedPortableSnapshotV1 {
    _exact_bootstrap: Zeroizing<Vec<u8>>,
    source_vault_id: VaultId,
    candidates: BTreeMap<ItemId, Vec<ItemCandidate>>,
    excluded_item_count: usize,
}

impl OpenedPortableSnapshotV1 {
    /// Return the number of distinct source item identities in the snapshot.
    pub fn item_count(&self) -> usize {
        self.candidates.len()
    }

    /// Return the number of retained current source candidates.
    pub fn candidate_count(&self) -> usize {
        self.candidates.values().map(Vec::len).sum()
    }

    /// Return how many source items this snapshot's own producer already
    /// excluded (VLT-PM05 §13.9).
    ///
    /// Always zero for an ordinary export. Nonzero only for a
    /// [`PortableExportCompletenessV1::BestEffort`] export that left at
    /// least one unencodable item out — an aggregate count, the same
    /// VLT-PM00 §20-permitted shape as [`Self::attachment_bearing_item_count`],
    /// not the excluded identities themselves: those were the *source*
    /// vault's items, not this (importing) operator's, so only their count
    /// crosses the cross-vault boundary this type exists to narrow.
    pub fn excluded_item_count(&self) -> usize {
        self.excluded_item_count
    }

    /// Return how many of this snapshot's items carry at least one attachment.
    ///
    /// The artifact encodes item state unmodified, so an attachment's *set
    /// membership* does travel — the chunk and manifest objects it names do
    /// not, because they are repository objects and this is a record snapshot.
    /// Import therefore drops the set (`remap_imported_item_state`) rather
    /// than producing an item that claims attachments no `attachment export`
    /// in the target could ever satisfy.
    ///
    /// This count exists so the import ceremony can *say* that, the way the
    /// export ceremony already does. It is an aggregate and nothing else — no
    /// identity, name, or size — which is what VLT-PM00 §20 permits a host to
    /// observe. VLT-PM47 §8.3.
    pub fn attachment_bearing_item_count(&self) -> usize {
        self.candidates
            .values()
            .filter(|candidates| {
                candidates.iter().any(|candidate| {
                    matches!(candidate.state(), ItemState::Live(document)
                        if !document.attachments().is_empty())
                })
            })
            .count()
    }

    /// Bind the complete authenticated source semantics for later restore verification.
    ///
    /// The opaque result retains source identities only to prove cross-vault
    /// disjointness and exposes no record, schema, timestamp, or field value.
    pub fn prepare_restore_verification(
        &self,
    ) -> Result<crate::PortableRestoreExpectationV1, ApplicationError> {
        crate::PortableRestoreExpectationV1::from_source(self.source_vault_id, &self.candidates)
    }

    pub(crate) fn into_import_parts(self) -> (VaultId, BTreeMap<ItemId, Vec<ItemCandidate>>) {
        let Self {
            source_vault_id,
            candidates,
            ..
        } = self;
        (source_vault_id, candidates)
    }
}

impl Debug for OpenedPortableSnapshotV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpenedPortableSnapshotV1(<redacted>)")
    }
}

/// Authenticate, decrypt, and strictly validate one untrusted portable artifact.
///
/// The caller supplies an owned separately collected passphrase and an explicit
/// Argon2id resource ceiling. Authentication completes before any plaintext is
/// parsed. The returned snapshot intentionally exposes only aggregate counts.
pub fn open_portable_with_passphrase(
    artifact: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
    policy: PortableOpenPolicyV1,
) -> Result<OpenedPortableSnapshotV1, ApplicationError> {
    if passphrase.is_empty() || passphrase.len() > MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES {
        return Err(ApplicationError::InvalidInput);
    }
    if artifact.len() > MAX_PORTABLE_EXPORT_ARTIFACT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }

    let mut fields =
        SecretCborValue::new(decode(artifact).map_err(|_| ApplicationError::IntegrityFailure)?)
            .into_map()?;
    fields.require_keys(&[1, 2, 3, 4, 5, 6, 7])?;
    check_wire_value(fields.take(1)?.into_uint()?, VERSION)?;
    check_wire_value(fields.take(2)?.into_uint()?, PASSPHRASE_PROTECTION)?;
    check_wire_value(fields.take(3)?.into_uint()?, CRYPTO_SUITE_V1.into())?;
    let mut kdf_fields = fields.take(4)?.into_map()?;
    kdf_fields.require_keys(&[1, 2, 3, 4])?;
    let kdf = Argon2idParametersV1 {
        memory_kib: u32::try_from(kdf_fields.take(1)?.into_uint()?)
            .map_err(|_| ApplicationError::BoundExceeded)?,
        iterations: u32::try_from(kdf_fields.take(2)?.into_uint()?)
            .map_err(|_| ApplicationError::BoundExceeded)?,
        lanes: u8::try_from(kdf_fields.take(3)?.into_uint()?)
            .map_err(|_| ApplicationError::BoundExceeded)?,
        salt: kdf_fields.take(4)?.into_fixed()?,
    };
    kdf.validate()
        .map_err(|_| ApplicationError::BoundExceeded)?;
    if !policy.allows(&kdf) {
        return Err(ApplicationError::BoundExceeded);
    }
    let nonce = fields.take(5)?.into_fixed()?;
    let ciphertext = Zeroizing::new(fields.take(6)?.into_bytes()?);
    if ciphertext.len() > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    let tag = fields.take(7)?.into_fixed()?;

    let derived = Zeroizing::new(
        argon2id(
            &passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .map_err(|_| ApplicationError::InvalidInput)?,
    );
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&derived);
    let plaintext = Zeroizing::new(
        xchacha20_poly1305_aead_decrypt(
            &ciphertext,
            &key,
            &nonce,
            &artifact_aad(&kdf, nonce),
            &tag,
        )
        .ok_or(ApplicationError::AuthenticationFailed)?,
    );
    if plaintext.len() > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }
    parse_opened_snapshot(&plaintext)
}

fn encrypt_portable_plaintext(
    plaintext: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
    kdf: &Argon2idParametersV1,
    nonce: [u8; NONCE_BYTES],
) -> Result<PortableExportArtifactV1, ApplicationError> {
    let derived = Zeroizing::new(
        argon2id(
            &passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .map_err(|_| ApplicationError::InvalidInput)?,
    );
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&derived);
    let aad = artifact_aad(kdf, nonce);
    let (ciphertext, tag) = xchacha20_poly1305_aead_encrypt(plaintext, &key, &nonce, &aad);
    let artifact = CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(PASSPHRASE_PROTECTION)),
        field(3, CborValue::Unsigned(CRYPTO_SUITE_V1.into())),
        field(4, kdf_value(kdf)),
        field(5, CborValue::Bytes(nonce.to_vec())),
        field(6, CborValue::Bytes(ciphertext)),
        field(7, CborValue::Bytes(tag.to_vec())),
    ]);
    // The artifact carries the whole export ciphertext, which this
    // module bounds at MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES (512 MiB) --
    // five hundred times canonical-CBOR's 1 MiB encoded ceiling. Any
    // export past that ceiling therefore has to be reported rather than
    // aborting the process, and unlike the record encodes this needs no
    // hostile peer at all: an ordinary vault of a few dozen items
    // produces an artifact larger than 1 MiB.
    Ok(PortableExportArtifactV1 {
        bytes: try_encode(&artifact).map_err(crate::codec::map_encode_error)?,
    })
}

fn parse_opened_snapshot(plaintext: &[u8]) -> Result<OpenedPortableSnapshotV1, ApplicationError> {
    let mut fields =
        SecretCborValue::new(decode(plaintext).map_err(|_| ApplicationError::IntegrityFailure)?)
            .into_map()?;
    // Field 6 (`excluded_item_ids`, VLT-PM05 §13.9) is optional on read: a
    // snapshot from before this field existed has exactly five fields and
    // reads exactly as it always did. A writer only ever emits field 6 when
    // it is non-empty (see `export_portable_with_passphrase`), so an
    // ordinary export today is still byte-for-byte the five-field shape a
    // pre-§13.9 reader already understands.
    let has_excluded = fields.contains_key(6);
    let required_keys: &[u64] = if has_excluded {
        &[1, 2, 3, 4, 5, 6]
    } else {
        &[1, 2, 3, 4, 5]
    };
    fields.require_keys(required_keys)?;
    check_wire_value(fields.take(1)?.into_uint()?, VERSION)?;
    let exact_bootstrap = Zeroizing::new(fields.take(2)?.into_bytes()?);
    let entries_value = fields.take(3)?;
    // Re-encoding entries that arrived inside someone else's export: the
    // same shape as `decode_record`'s opaque arm. The producer's framing
    // budget need not be the encoder's ceiling, so a snapshot that
    // decodes here may still be one the encoder declines to re-emit.
    // Failing closed rejects one import; panicking loses the process.
    let encoded_entries =
        Zeroizing::new(try_encode(entries_value.get()).map_err(crate::codec::map_encode_error)?);
    let candidate_count = usize::try_from(fields.take(4)?.into_uint()?)
        .map_err(|_| ApplicationError::BoundExceeded)?;
    let expected_hash: [u8; 32] = fields.take(5)?.into_fixed()?;
    if snapshot_hash(&exact_bootstrap, &encoded_entries)? != expected_hash {
        return Err(ApplicationError::IntegrityFailure);
    }
    let source_bootstrap = verify_signed_bootstrap(&exact_bootstrap)?;
    // `excluded_item_ids` is deliberately outside `snapshot_hash`'s own
    // domain, the same shape `candidate_count` already has: both are
    // separately, structurally cross-checked below rather than folded into
    // the hash, and both are still covered end to end by the outer AEAD
    // tag over the complete plaintext (`open_portable_with_passphrase`).
    let excluded_item_ids = if has_excluded {
        parse_excluded_item_ids(fields.take(6)?)?
    } else {
        Vec::new()
    };

    let mut entries = entries_value.into_values()?;
    if entries.len() != candidate_count {
        return Err(ApplicationError::IntegrityFailure);
    }
    if candidate_count > MAX_CATALOG_ENTRIES * MAX_CANDIDATES_PER_ITEM {
        return Err(ApplicationError::BoundExceeded);
    }
    let mut candidates: BTreeMap<ItemId, Vec<ItemCandidate>> = BTreeMap::new();
    let mut next_identity = None;
    while let Some(entry) = entries.pop() {
        let mut entry = entry.into_map()?;
        entry.require_keys(&[1, 2, 3])?;
        let item_id = ItemId::new(entry.take(1)?.into_fixed()?);
        let revision_id = RevisionId::new(entry.take(2)?.into_fixed()?);
        let identity = (item_id, revision_id);
        if next_identity.is_some_and(|next| identity >= next) {
            return Err(ApplicationError::IntegrityFailure);
        }
        next_identity = Some(identity);
        let encoded_revision = Zeroizing::new(entry.take(3)?.into_bytes()?);
        let candidate = decode_item_revision(revision_id, &encoded_revision)?;
        if candidate.item_id() != item_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        if !candidates.contains_key(&item_id) && candidates.len() == MAX_CATALOG_ENTRIES {
            return Err(ApplicationError::BoundExceeded);
        }
        let item_candidates = candidates.entry(item_id).or_default();
        if item_candidates.len() == MAX_CANDIDATES_PER_ITEM {
            return Err(ApplicationError::BoundExceeded);
        }
        item_candidates.push(candidate);
    }
    for item_candidates in candidates.values_mut() {
        item_candidates.reverse();
    }

    // A producer must not claim an item both present and excluded -- that
    // has no honest interpretation (VLT-PM05 §13.9's own writer never
    // produces it, since a poisoned item's candidates are never partially
    // committed to `entries`; see `export_portable_with_passphrase`).
    if excluded_item_ids
        .iter()
        .any(|item_id| candidates.contains_key(item_id))
    {
        return Err(ApplicationError::IntegrityFailure);
    }

    Ok(OpenedPortableSnapshotV1 {
        _exact_bootstrap: exact_bootstrap,
        source_vault_id: source_bootstrap.vault_id,
        candidates,
        excluded_item_count: excluded_item_ids.len(),
    })
}

/// Decode field 6's array of excluded source `ItemId`s.
///
/// Bounded before allocation the same way `candidate_count` already is
/// (`MAX_CATALOG_ENTRIES`, since no vault can have more distinct items than
/// that regardless of how many are excluded) and rejects a duplicate the
/// same way the candidate loop above rejects an out-of-order identity: a
/// well-behaved producer never emits one twice.
fn parse_excluded_item_ids(value: SecretCborValue) -> Result<Vec<ItemId>, ApplicationError> {
    let mut raw = value.into_values()?;
    if raw.len() > MAX_CATALOG_ENTRIES {
        return Err(ApplicationError::BoundExceeded);
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(raw.len());
    while let Some(value) = raw.pop() {
        let item_id = ItemId::new(value.into_fixed()?);
        if !seen.insert(item_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        ids.push(item_id);
    }
    Ok(ids)
}

fn check_wire_value(actual: u64, expected: u64) -> Result<(), ApplicationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ApplicationError::Unsupported)
    }
}

/// Build one canonical authenticated encrypted snapshot of `candidates`.
///
/// Every current live, tombstone, and conflicted candidate is included
/// under [`PortableExportCompletenessV1::Strict`] — the guarantee this
/// function has always made — or every candidate this module can re-encode
/// is included under [`PortableExportCompletenessV1::BestEffort`], with the
/// remainder named in the returned outcome's `excluded_item_ids` (VLT-PM05
/// §13.9).
///
/// An item is excluded as a whole, never as a partial candidate set: if any
/// one of an item's current candidates cannot be re-encoded, every candidate
/// of that item is left out, including ones that individually would have
/// encoded fine. A conflicted item with one small candidate and one oversized
/// one is not split into "the small one travels, the large one doesn't" —
/// that would silently hand the target a single, unconflicted candidate
/// where the source actually had an unresolved conflict, misrepresenting the
/// item's own history rather than merely omitting it. Losing an item
/// entirely is an accounted-for gap; losing only its *conflictedness* would
/// be a silent correctness defect one layer worse.
pub(crate) fn export_portable_with_passphrase(
    candidates: &BTreeMap<ItemId, Vec<ItemCandidate>>,
    active: &crate::ActiveStateV1,
    exact_bootstrap: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
    policy: PortableExportPolicyV1,
    randomness: PortableExportRandomnessV1,
    completeness: PortableExportCompletenessV1,
) -> Result<PortableExportOutcomeV1, ApplicationError> {
    if passphrase.is_empty() || passphrase.len() > MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES {
        return Err(ApplicationError::InvalidInput);
    }
    verify_active_bootstrap(active, exact_bootstrap)?;

    let salt = randomness.salt();
    let nonce = randomness.nonce();
    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt,
    };
    kdf.validate().map_err(|_| ApplicationError::InvalidInput)?;

    let mut entries = SecretCborValues::default();
    let mut excluded_item_ids = Vec::new();
    let mut estimated_size = exact_bootstrap
        .len()
        .checked_add(SNAPSHOT_ESTIMATE_OVERHEAD)
        .ok_or(ApplicationError::BoundExceeded)?;
    for (item_id, item_candidates) in candidates {
        let mut ordered = item_candidates.iter().collect::<Vec<_>>();
        ordered.sort_unstable_by_key(|candidate| candidate.revision_id());
        let mut item_entries = Vec::with_capacity(ordered.len());
        // Tentative running total including this item's own candidates.
        // Committed to `estimated_size` only once this item is known not
        // to be excluded (below) -- an item that ends up poisoned must not
        // have contributed to the ceiling check that decides every other
        // item's fate.
        let mut item_size = estimated_size;
        let mut poisoned = false;
        for candidate in ordered {
            if candidate.item_id() != *item_id {
                zeroize_cbor_values(&mut item_entries);
                return Err(ApplicationError::IntegrityFailure);
            }
            let revision = match encode_item_revision(candidate.causal_parents(), candidate.state())
            {
                Ok(revision) => Zeroizing::new(revision),
                Err(ApplicationError::BoundExceeded)
                    if completeness == PortableExportCompletenessV1::BestEffort =>
                {
                    // Strict propagates this Err unmodified below; only
                    // BestEffort takes this arm and keeps going.
                    poisoned = true;
                    break;
                }
                Err(error) => {
                    zeroize_cbor_values(&mut item_entries);
                    return Err(error);
                }
            };
            item_size = item_size
                .checked_add(revision.len())
                .and_then(|size| size.checked_add(CANDIDATE_ESTIMATE_OVERHEAD))
                .ok_or(ApplicationError::BoundExceeded)?;
            item_entries.push(CborValue::Map(vec![
                field(1, bytes(item_id.as_bytes())),
                field(2, bytes(candidate.revision_id().as_bytes())),
                field(3, CborValue::Bytes(revision.into_inner())),
            ]));
        }
        if poisoned {
            zeroize_cbor_values(&mut item_entries);
            excluded_item_ids.push(*item_id);
        } else {
            if item_size > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
                zeroize_cbor_values(&mut item_entries);
                return Err(ApplicationError::BoundExceeded);
            }
            estimated_size = item_size;
            entries.extend(item_entries);
        }
    }

    let candidate_count =
        u64::try_from(entries.len()).map_err(|_| ApplicationError::BoundExceeded)?;
    let mut entries_value = SecretCborValue::new(CborValue::Array(entries.into_inner()));
    // Every candidate revision in the vault, concatenated. This module's
    // own ceiling on it is 512 MiB; the encoder's is 1 MiB, so a vault of
    // a few dozen ordinary items already crosses the tighter one.
    let encoded_entries =
        Zeroizing::new(try_encode(entries_value.get()).map_err(crate::codec::map_encode_error)?);
    let snapshot_hash = snapshot_hash(exact_bootstrap, &encoded_entries)?;
    let mut snapshot_fields = vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Bytes(exact_bootstrap.to_vec())),
        field(3, entries_value.take()),
        field(4, CborValue::Unsigned(candidate_count)),
        field(5, CborValue::Bytes(snapshot_hash.to_vec())),
    ];
    // Field 6 is emitted only when non-empty, so an ordinary (or a
    // BestEffort export that happened not to need to exclude anything)
    // still produces the exact five-field shape every prior version of
    // this module already writes and reads -- byte-for-byte unchanged, and
    // still readable by a `vault-pm` build from before this field existed.
    if !excluded_item_ids.is_empty() {
        snapshot_fields.push(field(
            6,
            CborValue::Array(
                excluded_item_ids
                    .iter()
                    .map(|item_id| bytes(item_id.as_bytes()))
                    .collect(),
            ),
        ));
    }
    let snapshot = SecretCborValue::new(CborValue::Map(snapshot_fields));
    let plaintext =
        Zeroizing::new(try_encode(snapshot.get()).map_err(crate::codec::map_encode_error)?);
    if plaintext.len() > MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES {
        return Err(ApplicationError::BoundExceeded);
    }

    let artifact = encrypt_portable_plaintext(&plaintext, passphrase, &kdf, nonce)?;
    Ok(PortableExportOutcomeV1 {
        artifact,
        excluded_item_ids,
    })
}

#[cfg(test)]
pub(crate) fn encrypt_portable_for_test(
    plaintext: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
    policy: PortableExportPolicyV1,
    randomness: PortableExportRandomnessV1,
) -> PortableExportArtifactV1 {
    let nonce = randomness.nonce();
    let kdf = Argon2idParametersV1 {
        memory_kib: policy.memory_kib,
        iterations: policy.iterations,
        lanes: policy.lanes,
        salt: randomness.salt(),
    };
    encrypt_portable_plaintext(plaintext, passphrase, &kdf, nonce).unwrap()
}

pub(crate) fn snapshot_hash(
    exact_bootstrap: &[u8],
    encoded_entries: &[u8],
) -> Result<[u8; 32], ApplicationError> {
    let bootstrap_len =
        u64::try_from(exact_bootstrap.len()).map_err(|_| ApplicationError::BoundExceeded)?;
    let mut preimage = Zeroizing::new(Vec::with_capacity(
        SNAPSHOT_HASH_DOMAIN.len() + 8 + exact_bootstrap.len() + encoded_entries.len(),
    ));
    preimage.extend_from_slice(SNAPSHOT_HASH_DOMAIN);
    preimage.extend_from_slice(&bootstrap_len.to_be_bytes());
    preimage.extend_from_slice(exact_bootstrap);
    preimage.extend_from_slice(encoded_entries);
    Ok(sha256(&preimage))
}

fn artifact_aad(kdf: &Argon2idParametersV1, nonce: [u8; NONCE_BYTES]) -> Vec<u8> {
    let header = CborValue::Map(vec![
        field(1, CborValue::Unsigned(VERSION)),
        field(2, CborValue::Unsigned(PASSPHRASE_PROTECTION)),
        field(3, CborValue::Unsigned(CRYPTO_SUITE_V1.into())),
        field(4, kdf_value(kdf)),
        field(5, CborValue::Bytes(nonce.to_vec())),
    ]);
    // The one encode left infallible in this module, and provably so:
    // the header is a version, two small enum codes, four Argon2id
    // integers, a 16-byte salt, and a 24-byte nonce. Its size is the same
    // on every call and nowhere near the encoder's ceiling or depth cap,
    // so there is no oversized input for a checked encode to report.
    let encoded = encode(&header);
    let mut aad = Vec::with_capacity(EXPORT_AAD_DOMAIN.len() + encoded.len());
    aad.extend_from_slice(EXPORT_AAD_DOMAIN);
    aad.extend_from_slice(&encoded);
    aad
}

fn kdf_value(kdf: &Argon2idParametersV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(kdf.memory_kib.into())),
        field(2, CborValue::Unsigned(kdf.iterations.into())),
        field(3, CborValue::Unsigned(kdf.lanes.into())),
        field(4, CborValue::Bytes(kdf.salt.to_vec())),
    ])
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes<const N: usize>(value: &[u8; N]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn zeroize_cbor_values(values: &mut [CborValue]) {
    for value in values {
        zeroize_cbor(value);
    }
}

#[derive(Default)]
struct SecretCborValues(Option<Vec<CborValue>>);

impl SecretCborValues {
    /// Move every value out of `values` and append it here.
    ///
    /// Takes ownership rather than borrowing so an already-built `Vec` of
    /// one item's successfully-encoded candidates can be folded in without a
    /// second copy -- used by `export_portable_with_passphrase` once an
    /// item is known not to be excluded. `values` is a plain `Vec`, not
    /// another `SecretCborValues`: it is always either moved in here or
    /// explicitly zeroized by the caller first (the poisoned-item path),
    /// never dropped un-wiped either way.
    fn extend(&mut self, values: Vec<CborValue>) {
        self.0.get_or_insert_with(Vec::new).extend(values);
    }

    fn len(&self) -> usize {
        self.0.as_ref().map_or(0, Vec::len)
    }

    fn into_inner(mut self) -> Vec<CborValue> {
        self.0.take().unwrap_or_default()
    }

    fn pop(&mut self) -> Option<SecretCborValue> {
        self.0.as_mut()?.pop().map(SecretCborValue::new)
    }
}

impl Drop for SecretCborValues {
    fn drop(&mut self) {
        if let Some(values) = &mut self.0 {
            zeroize_cbor_values(values);
        }
    }
}

struct SecretCborValue(Option<CborValue>);

impl SecretCborValue {
    fn new(value: CborValue) -> Self {
        Self(Some(value))
    }

    fn get(&self) -> &CborValue {
        self.0.as_ref().expect("secret CBOR value is present")
    }

    fn take(&mut self) -> CborValue {
        self.0.take().expect("secret CBOR value is present")
    }

    fn into_map(mut self) -> Result<SecretCborMap, ApplicationError> {
        match self.take() {
            CborValue::Map(entries) => Ok(SecretCborMap::new(entries)),
            mut value => {
                zeroize_cbor(&mut value);
                Err(ApplicationError::IntegrityFailure)
            }
        }
    }

    fn into_values(mut self) -> Result<SecretCborValues, ApplicationError> {
        match self.take() {
            CborValue::Array(values) => Ok(SecretCborValues(Some(values))),
            mut value => {
                zeroize_cbor(&mut value);
                Err(ApplicationError::IntegrityFailure)
            }
        }
    }

    fn into_uint(mut self) -> Result<u64, ApplicationError> {
        match self.take() {
            CborValue::Unsigned(value) => Ok(value),
            mut value => {
                zeroize_cbor(&mut value);
                Err(ApplicationError::IntegrityFailure)
            }
        }
    }

    fn into_bytes(mut self) -> Result<Vec<u8>, ApplicationError> {
        match self.take() {
            CborValue::Bytes(value) => Ok(value),
            mut value => {
                zeroize_cbor(&mut value);
                Err(ApplicationError::IntegrityFailure)
            }
        }
    }

    fn into_fixed<const N: usize>(self) -> Result<[u8; N], ApplicationError> {
        let mut value = self.into_bytes()?;
        let result = value
            .as_slice()
            .try_into()
            .map_err(|_| ApplicationError::IntegrityFailure);
        value.zeroize();
        result
    }
}

impl Drop for SecretCborValue {
    fn drop(&mut self) {
        if let Some(value) = &mut self.0 {
            zeroize_cbor(value);
        }
    }
}

struct SecretCborMap(Option<Vec<(CborValue, CborValue)>>);

impl SecretCborMap {
    fn new(entries: Vec<(CborValue, CborValue)>) -> Self {
        Self(Some(entries))
    }

    fn require_keys(&self, expected: &[u64]) -> Result<(), ApplicationError> {
        let entries = self.0.as_ref().expect("secret CBOR map is present");
        if entries.len() != expected.len()
            || entries.iter().any(
                |(key, _)| !matches!(key, CborValue::Unsigned(value) if expected.contains(value)),
            )
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(())
    }

    /// Return whether `key` is present, without removing it.
    ///
    /// Used to decide *which* `require_keys` shape to demand (VLT-PM05
    /// §13.9's optional field 6) before that exact-arity check runs --
    /// `require_keys` itself intentionally has no "optional" mode of its
    /// own, so the caller resolves the ambiguity first and picks the
    /// matching fixed key set.
    fn contains_key(&self, key: u64) -> bool {
        let entries = self.0.as_ref().expect("secret CBOR map is present");
        entries
            .iter()
            .any(|(candidate, _)| candidate == &CborValue::Unsigned(key))
    }

    fn take(&mut self, key: u64) -> Result<SecretCborValue, ApplicationError> {
        let entries = self.0.as_mut().expect("secret CBOR map is present");
        let index = entries
            .iter()
            .position(|(candidate, _)| candidate == &CborValue::Unsigned(key))
            .ok_or(ApplicationError::IntegrityFailure)?;
        let (mut encoded_key, value) = entries.remove(index);
        zeroize_cbor(&mut encoded_key);
        Ok(SecretCborValue::new(value))
    }
}

impl Drop for SecretCborMap {
    fn drop(&mut self) {
        if let Some(entries) = &mut self.0 {
            for (key, value) in entries {
                zeroize_cbor(key);
                zeroize_cbor(value);
            }
        }
    }
}

pub(crate) fn zeroize_cbor(value: &mut CborValue) {
    match value {
        CborValue::Bytes(bytes) => bytes.zeroize(),
        CborValue::Text(text) => text.zeroize(),
        CborValue::Array(values) => zeroize_cbor_values(values),
        CborValue::Map(entries) => {
            for (key, value) in entries {
                zeroize_cbor(key);
                zeroize_cbor(value);
            }
        }
        CborValue::Tag(_, value) => zeroize_cbor(value),
        CborValue::Unsigned(_) | CborValue::Negative(_) | CborValue::Bool(_) | CborValue::Null => {}
    }
}

#[cfg(test)]
pub(crate) fn decrypt_portable_for_test(
    artifact: &[u8],
    passphrase: Zeroizing<Vec<u8>>,
) -> Option<Zeroizing<Vec<u8>>> {
    use coding_adventures_canonical_cbor::decode;
    use coding_adventures_chacha20_poly1305::xchacha20_poly1305_aead_decrypt;

    let CborValue::Map(mut fields) = decode(artifact).ok()? else {
        return None;
    };
    let version = take_test_uint(&mut fields, 1)?;
    let protection = take_test_uint(&mut fields, 2)?;
    let suite = take_test_uint(&mut fields, 3)?;
    if version != VERSION
        || protection != PASSPHRASE_PROTECTION
        || suite != u64::from(CRYPTO_SUITE_V1)
    {
        return None;
    }
    let CborValue::Map(mut kdf_fields) = take_test_field(&mut fields, 4)? else {
        return None;
    };
    let memory_kib = u32::try_from(take_test_uint(&mut kdf_fields, 1)?).ok()?;
    let iterations = u32::try_from(take_test_uint(&mut kdf_fields, 2)?).ok()?;
    let lanes = u8::try_from(take_test_uint(&mut kdf_fields, 3)?).ok()?;
    let salt = take_test_bytes(&mut kdf_fields, 4)?.try_into().ok()?;
    let kdf = Argon2idParametersV1 {
        memory_kib,
        iterations,
        lanes,
        salt,
    };
    kdf.validate().ok()?;
    let nonce = take_test_bytes(&mut fields, 5)?.try_into().ok()?;
    let ciphertext = take_test_bytes(&mut fields, 6)?;
    let tag = take_test_bytes(&mut fields, 7)?.try_into().ok()?;
    if !fields.is_empty() || !kdf_fields.is_empty() {
        return None;
    }
    let derived = Zeroizing::new(
        argon2id(
            &passphrase,
            &kdf.salt,
            kdf.iterations,
            kdf.memory_kib,
            kdf.lanes.into(),
            32,
            &Argon2idOptions::default(),
        )
        .ok()?,
    );
    let mut key = Zeroizing::new([0; 32]);
    key.copy_from_slice(&derived);
    xchacha20_poly1305_aead_decrypt(&ciphertext, &key, &nonce, &artifact_aad(&kdf, nonce), &tag)
        .map(Zeroizing::new)
}

#[cfg(test)]
fn take_test_field(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<CborValue> {
    let index = fields
        .iter()
        .position(|(candidate, _)| candidate == &CborValue::Unsigned(key))?;
    Some(fields.remove(index).1)
}

#[cfg(test)]
fn take_test_uint(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<u64> {
    match take_test_field(fields, key)? {
        CborValue::Unsigned(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
fn take_test_bytes(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> Option<Vec<u8>> {
    match take_test_field(fields, key)? {
        CborValue::Bytes(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_randomness_and_artifact_diagnostics_are_closed() {
        let policy = PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        assert_eq!(policy.memory_kib(), 8 * 1024);
        assert_eq!(policy.iterations(), 1);
        assert_eq!(policy.lanes(), 1);
        assert_eq!(
            format!("{policy:?}"),
            "PortableExportPolicyV1 { memory_kib: 8192, iterations: 1, lanes: 1 }"
        );
        assert_eq!(
            PortableExportPolicyV1::new(1, 1, 1),
            Err(ApplicationError::InvalidInput)
        );
        let open_policy = PortableOpenPolicyV1::new(8 * 1024, 2, 1).unwrap();
        assert_eq!(
            format!("{open_policy:?}"),
            "PortableOpenPolicyV1 { max_memory_kib: 8192, max_iterations: 2, max_lanes: 1 }"
        );
        assert_eq!(
            PortableOpenPolicyV1::new(1, 1, 1),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            MAX_PORTABLE_EXPORT_ARTIFACT_BYTES,
            MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES + 4_096
        );

        let randomness = PortableExportRandomnessV1::new([0x5a; PORTABLE_EXPORT_RANDOM_BYTES]);
        assert_eq!(
            format!("{randomness:?}"),
            "PortableExportRandomnessV1(<redacted>)"
        );
        let artifact = PortableExportArtifactV1 {
            bytes: vec![1, 2, 3],
        };
        assert_eq!(
            format!("{artifact:?}"),
            "PortableExportArtifactV1(<encrypted>)"
        );
        assert_eq!(artifact.as_bytes(), &[1, 2, 3]);
        assert_eq!(artifact.into_bytes(), vec![1, 2, 3]);
    }

    #[test]
    fn recursive_cbor_wipe_clears_secret_shaped_allocations() {
        let mut value = CborValue::Map(vec![field(
            1,
            CborValue::Array(vec![
                CborValue::Bytes(b"secret bytes".to_vec()),
                CborValue::Text("secret text".to_owned()),
                CborValue::Tag(7, Box::new(CborValue::Bytes(vec![9; 8]))),
            ]),
        )]);
        zeroize_cbor(&mut value);
        let CborValue::Map(entries) = value else {
            panic!()
        };
        let CborValue::Array(values) = &entries[0].1 else {
            panic!()
        };
        assert_eq!(values[0], CborValue::Bytes(Vec::new()));
        assert_eq!(values[1], CborValue::Text(String::new()));
        assert_eq!(
            values[2],
            CborValue::Tag(7, Box::new(CborValue::Bytes(Vec::new())))
        );
    }
}
