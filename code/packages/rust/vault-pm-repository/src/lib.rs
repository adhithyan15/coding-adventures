//! VLT-PM04 verified immutable repository for the local-first password manager.
//!
//! The repository composes one injected opaque object store with one mandatory
//! cryptographic verifier. It owns publication ordering, read-back checks,
//! signed discovery, commit-DAG validation, local pins, history, and GC plans.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_hmac::hmac_sha256;
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_format::{
    AnnouncementV1, CommitV1, DeviceId, ObjectFrameV1, ObjectId as FormatObjectId, VaultId,
};
use coding_adventures_vault_pm_storage::{
    BucketId, ListCursor, ObjectBytes, ObjectId as StorageObjectId, StoreError, VaultLocator,
    VaultObjectStore,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Mutex;

const STORE_LOCATOR_LABEL: &[u8] = b"vpm/store-locator/v1";
const OBJECT_BUCKET_LABEL: &[u8] = b"vpm/objects/v1";
const ANNOUNCEMENT_BUCKET_LABEL: &[u8] = b"vpm/announcements/v1";
const ANNOUNCEMENT_ID_DOMAIN: &[u8] = b"VPM-ANNOUNCEMENT-ID-v1";

/// Maximum encrypted object frames supplied by one publication.
pub const MAX_PUBLICATION_OBJECTS: usize = 4_096;
/// Maximum signed announcements accepted by one complete discovery scan.
pub const MAX_ANNOUNCEMENTS: usize = 16_384;
/// Maximum commits expanded by one verified graph walk.
pub const MAX_GRAPH_COMMITS: usize = 65_536;
/// Maximum caller-owned commit head pins.
pub const MAX_HEAD_PINS: usize = 256;
/// Maximum ancestry summaries returned by one history request.
pub const MAX_HISTORY_ENTRIES: usize = 4_096;
/// Maximum object entries considered by one garbage-collection plan.
pub const MAX_GC_OBJECTS: usize = 131_072;
/// Page size used for complete storage listings.
pub const REPOSITORY_LIST_PAGE_SIZE: usize = 1_000;

/// Closed verifier failure. Implementations must not attach controlled text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VerificationError;

impl Display for VerificationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("repository verification failed")
    }
}

impl std::error::Error for VerificationError {}

/// Mandatory unlocked cryptographic verification boundary.
pub trait RepositoryVerifier: Send + Sync {
    /// Authenticate, decrypt, decode, authorize, and signature-verify a commit.
    fn verify_commit(
        &self,
        expected: &FormatObjectId,
        frame: &ObjectFrameV1,
    ) -> Result<CommitV1, VerificationError>;

    /// Decode, authorize, and signature-verify one signed announcement.
    fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError>;
}

/// Closed repository error taxonomy with payload-free diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepositoryError {
    /// Repository initialization has not completed.
    NotInitialized,
    /// Caller input violates a repository precondition.
    InvalidInput,
    /// A fixed repository bound would be exceeded.
    BoundExceeded,
    /// The injected storage operation failed.
    Storage,
    /// The mandatory cryptographic verifier rejected an object.
    Verification,
    /// Persisted bytes or cross-object relations are corrupt.
    Corruption,
    /// A pinned or referenced object is absent from the provider view.
    ProviderWithholding,
    /// One device counter names two different commits.
    DeviceEquivocation,
    /// The verified parent relation contains a cycle.
    GraphCycle,
    /// A caller attempted an unsafe empty or inconsistent pin operation.
    PinConflict,
}

impl RepositoryError {
    fn label(self) -> &'static str {
        match self {
            Self::NotInitialized => "NotInitialized",
            Self::InvalidInput => "InvalidInput",
            Self::BoundExceeded => "BoundExceeded",
            Self::Storage => "Storage",
            Self::Verification => "Verification",
            Self::Corruption => "Corruption",
            Self::ProviderWithholding => "ProviderWithholding",
            Self::DeviceEquivocation => "DeviceEquivocation",
            Self::GraphCycle => "GraphCycle",
            Self::PinConflict => "PinConflict",
        }
    }
}

impl Debug for RepositoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl Display for RepositoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("vault-pm-repository: ")?;
        formatter.write_str(self.label())
    }
}

impl std::error::Error for RepositoryError {}

/// Opaque store locator and repository bucket tuple derived from a locator key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RepositoryAddress {
    store_locator: VaultLocator,
    object_bucket: BucketId,
    announcement_bucket: BucketId,
}

impl RepositoryAddress {
    /// Derive the exact VLT-PM04 address tuple from a non-empty 32-byte key.
    pub fn derive(locator_key: &[u8; 32]) -> Self {
        Self {
            store_locator: VaultLocator::new(
                hmac_sha256(locator_key, STORE_LOCATOR_LABEL)
                    .expect("a fixed 32-byte HMAC key is non-empty"),
            ),
            object_bucket: BucketId::new(
                hmac_sha256(locator_key, OBJECT_BUCKET_LABEL)
                    .expect("a fixed 32-byte HMAC key is non-empty"),
            ),
            announcement_bucket: BucketId::new(
                hmac_sha256(locator_key, ANNOUNCEMENT_BUCKET_LABEL)
                    .expect("a fixed 32-byte HMAC key is non-empty"),
            ),
        }
    }

    /// Return the opaque store binding locator.
    pub const fn store_locator(&self) -> VaultLocator {
        self.store_locator
    }

    /// Return the opaque encrypted-object bucket.
    pub const fn object_bucket(&self) -> BucketId {
        self.object_bucket
    }

    /// Return the opaque signed-announcement bucket.
    pub const fn announcement_bucket(&self) -> BucketId {
        self.announcement_bucket
    }
}

impl Debug for RepositoryAddress {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("RepositoryAddress(<redacted>)")
    }
}

/// Derive the storage object ID for exact signed announcement bytes.
pub fn announcement_storage_id(bytes: &[u8]) -> StorageObjectId {
    let mut preimage = Vec::with_capacity(ANNOUNCEMENT_ID_DOMAIN.len() + bytes.len());
    preimage.extend_from_slice(ANNOUNCEMENT_ID_DOMAIN);
    preimage.extend_from_slice(bytes);
    StorageObjectId::new(sha256(&preimage))
}

/// Bounded sorted caller-owned set of accepted commit heads.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct PinnedHeads(BTreeSet<FormatObjectId>);

impl PinnedHeads {
    /// Construct and bound a pin set.
    pub fn new(heads: impl IntoIterator<Item = FormatObjectId>) -> Result<Self, RepositoryError> {
        let heads = heads.into_iter().collect::<BTreeSet<_>>();
        if heads.len() > MAX_HEAD_PINS {
            return Err(RepositoryError::BoundExceeded);
        }
        Ok(Self(heads))
    }

    /// Construct an empty fresh-device pin set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return whether this is an unanchored empty pin set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Return the number of retained heads.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate through exact pins in bytewise order.
    pub fn iter(&self) -> impl Iterator<Item = &FormatObjectId> {
        self.0.iter()
    }

    fn advanced(
        &self,
        commit: FormatObjectId,
        parents: &[FormatObjectId],
    ) -> Result<Self, RepositoryError> {
        let mut next = self.0.clone();
        for parent in parents {
            next.remove(parent);
        }
        next.insert(commit);
        Self::new(next)
    }
}

impl Debug for PinnedHeads {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedHeads")
            .field("count", &self.len())
            .finish()
    }
}

/// Complete already-encrypted and already-signed publication input.
pub struct Publication {
    objects: Vec<ObjectFrameV1>,
    commit: ObjectFrameV1,
    announcement: Vec<u8>,
}

impl Publication {
    /// Construct one publication batch for repository validation and commit.
    pub fn new(objects: Vec<ObjectFrameV1>, commit: ObjectFrameV1, announcement: Vec<u8>) -> Self {
        Self {
            objects,
            commit,
            announcement,
        }
    }
}

impl Debug for Publication {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Publication")
            .field("object_count", &self.objects.len())
            .field("commit", &"<redacted>")
            .field("announcement", &"<redacted>")
            .finish()
    }
}

/// Successful publication result returned only after exact read-back.
#[derive(Clone, PartialEq, Eq)]
pub struct PublicationReceipt {
    commit_id: FormatObjectId,
    heads: PinnedHeads,
}

impl PublicationReceipt {
    /// Return the newly published encrypted commit object ID.
    pub const fn commit_id(&self) -> FormatObjectId {
        self.commit_id
    }

    /// Return the safely advanced local head pins.
    pub fn heads(&self) -> &PinnedHeads {
        &self.heads
    }
}

impl Debug for PublicationReceipt {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicationReceipt")
            .field("commit_id", &"<redacted>")
            .field("head_count", &self.heads.len())
            .finish()
    }
}

/// Payload-free report from a complete verified repository open.
#[derive(Clone, PartialEq, Eq)]
pub struct OpenReport {
    heads: PinnedHeads,
    announcement_count: usize,
    commit_count: usize,
    fresh_device_unanchored: bool,
}

impl OpenReport {
    /// Return verified maximal commit heads.
    pub fn heads(&self) -> &PinnedHeads {
        &self.heads
    }

    /// Return the number of unique signed announcements verified.
    pub const fn announcement_count(&self) -> usize {
        self.announcement_count
    }

    /// Return the number of commits verified across complete ancestry.
    pub const fn commit_count(&self) -> usize {
        self.commit_count
    }

    /// Return whether no independent local pin anchored this provider view.
    pub const fn fresh_device_unanchored(&self) -> bool {
        self.fresh_device_unanchored
    }
}

impl Debug for OpenReport {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenReport")
            .field("head_count", &self.heads.len())
            .field("announcement_count", &self.announcement_count)
            .field("commit_count", &self.commit_count)
            .field("fresh_device_unanchored", &self.fresh_device_unanchored)
            .finish()
    }
}

/// Explicit ancestry metadata for one verified commit.
#[derive(Clone, PartialEq, Eq)]
pub struct CommitSummary {
    id: FormatObjectId,
    vault_id: VaultId,
    device_id: DeviceId,
    device_counter: u64,
    parents: Vec<FormatObjectId>,
    catalog_root: FormatObjectId,
    added_objects: Vec<FormatObjectId>,
    tombstone_root: Option<FormatObjectId>,
    device_certificate: FormatObjectId,
    wall_time_ms: u64,
}

impl CommitSummary {
    fn from_commit(id: FormatObjectId, commit: &CommitV1) -> Self {
        Self {
            id,
            vault_id: commit.vault_id,
            device_id: commit.device_id,
            device_counter: commit.device_counter,
            parents: commit.parents.clone(),
            catalog_root: commit.catalog_root,
            added_objects: commit.added_objects.clone(),
            tombstone_root: commit.tombstone_root,
            device_certificate: commit.device_certificate,
            wall_time_ms: commit.wall_time_ms,
        }
    }

    /// Return the encrypted commit object ID.
    pub const fn id(&self) -> FormatObjectId {
        self.id
    }

    /// Return the signed vault identity.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Return the signed writer device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Return the signed monotonic writer counter.
    pub const fn device_counter(&self) -> u64 {
        self.device_counter
    }

    /// Return signed parent commit IDs.
    pub fn parents(&self) -> &[FormatObjectId] {
        &self.parents
    }

    /// Return the signed encrypted catalog-root object ID.
    pub const fn catalog_root(&self) -> FormatObjectId {
        self.catalog_root
    }

    /// Return the signed set of newly reachable object IDs.
    pub fn added_objects(&self) -> &[FormatObjectId] {
        &self.added_objects
    }

    /// Return the optional signed encrypted tombstone-root object ID.
    pub const fn tombstone_root(&self) -> Option<FormatObjectId> {
        self.tombstone_root
    }

    /// Return the signed encrypted writer-certificate object ID.
    pub const fn device_certificate(&self) -> FormatObjectId {
        self.device_certificate
    }

    /// Return the signed advisory wall-clock value.
    pub const fn wall_time_ms(&self) -> u64 {
        self.wall_time_ms
    }
}

impl Debug for CommitSummary {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommitSummary")
            .field("id", &"<redacted>")
            .field("vault_id", &"<redacted>")
            .field("device_id", &"<redacted>")
            .field("device_counter", &self.device_counter)
            .field("parent_count", &self.parents.len())
            .field("added_object_count", &self.added_objects.len())
            .field("has_tombstone", &self.tombstone_root.is_some())
            .field("wall_time_ms", &self.wall_time_ms)
            .finish()
    }
}

/// Exact hash-verified encrypted object with redacted ordinary diagnostics.
#[derive(Clone, PartialEq, Eq)]
pub struct VerifiedObject {
    id: FormatObjectId,
    frame: ObjectFrameV1,
}

impl VerifiedObject {
    /// Return the verified VLT-PM01 object ID.
    pub const fn id(&self) -> FormatObjectId {
        self.id
    }

    /// Explicitly borrow the verified encrypted frame.
    pub const fn frame(&self) -> &ObjectFrameV1 {
        &self.frame
    }

    /// Explicitly consume this wrapper and return the encrypted frame.
    pub fn into_frame(self) -> ObjectFrameV1 {
        self.frame
    }
}

impl Debug for VerifiedObject {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedObject(<redacted>)")
    }
}

/// Conservative plan-only object reachability report.
#[derive(Clone, PartialEq, Eq)]
pub struct GcPlan {
    listed: usize,
    reachable: BTreeSet<FormatObjectId>,
    unreachable: BTreeSet<FormatObjectId>,
}

impl GcPlan {
    /// Return the number of objects listed during the complete scan.
    pub const fn listed_count(&self) -> usize {
        self.listed
    }

    /// Return exact retained object IDs.
    pub fn reachable(&self) -> &BTreeSet<FormatObjectId> {
        &self.reachable
    }

    /// Return exact plan-only deletion candidates.
    pub fn unreachable(&self) -> &BTreeSet<FormatObjectId> {
        &self.unreachable
    }
}

impl Debug for GcPlan {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GcPlan")
            .field("listed", &self.listed)
            .field("reachable_count", &self.reachable.len())
            .field("unreachable_count", &self.unreachable.len())
            .finish()
    }
}

struct VerifiedGraph {
    commits: BTreeMap<FormatObjectId, CommitV1>,
}

/// Verified immutable repository over injected storage and cryptography.
pub struct Repository<S: VaultObjectStore, V: RepositoryVerifier> {
    store: S,
    verifier: V,
    address: RepositoryAddress,
    initialized: Mutex<bool>,
}

impl<S: VaultObjectStore, V: RepositoryVerifier> Repository<S, V> {
    /// Construct an uninitialized repository composition.
    pub fn new(store: S, verifier: V, address: RepositoryAddress) -> Self {
        Self {
            store,
            verifier,
            address,
            initialized: Mutex::new(false),
        }
    }

    /// Initialize and bind the injected store to this repository address.
    pub fn initialize(&self) -> Result<(), RepositoryError> {
        self.store
            .initialize(&self.address.store_locator)
            .map_err(map_store_error)?;
        *self
            .initialized
            .lock()
            .map_err(|_| RepositoryError::Storage)? = true;
        Ok(())
    }

    /// Validate, publish, reread, and safely advance one immutable commit.
    pub fn publish(
        &self,
        publication: Publication,
        current_heads: &PinnedHeads,
    ) -> Result<PublicationReceipt, RepositoryError> {
        self.ensure_initialized()?;
        if publication.objects.len() > MAX_PUBLICATION_OBJECTS {
            return Err(RepositoryError::BoundExceeded);
        }

        let mut supplied = BTreeMap::new();
        for frame in publication.objects {
            let encoded = frame.encode().map_err(|_| RepositoryError::Corruption)?;
            let id = frame.id().map_err(|_| RepositoryError::Corruption)?;
            if supplied.insert(id, (frame, encoded)).is_some() {
                return Err(RepositoryError::InvalidInput);
            }
        }
        let commit_bytes = publication
            .commit
            .encode()
            .map_err(|_| RepositoryError::Corruption)?;
        let commit_id = publication
            .commit
            .id()
            .map_err(|_| RepositoryError::Corruption)?;
        if supplied.contains_key(&commit_id) {
            return Err(RepositoryError::InvalidInput);
        }
        let commit = self.verify_commit_frame(&commit_id, &publication.commit)?;
        let announcement = self.verify_announcement_bytes(&publication.announcement)?;
        Self::cross_check(&commit_id, &commit, &announcement)?;
        let next_heads = current_heads.advanced(commit_id, &commit.parents)?;

        let required = referenced_objects(&commit);
        if supplied.keys().any(|id| !required.contains(id)) {
            return Err(RepositoryError::InvalidInput);
        }
        for id in &required {
            if !supplied.contains_key(id) {
                self.fetch_frame(id)?;
            }
        }
        let mut parent_graph = self.load_graph(commit.parents.iter().copied())?;
        parent_graph.commits.insert(commit_id, commit.clone());
        validate_graph(&parent_graph)?;

        for (id, (_, encoded)) in &supplied {
            self.put_frame_exact(id, encoded)?;
        }
        self.put_frame_exact(&commit_id, &commit_bytes)?;
        let reread_commit = self.fetch_commit(&commit_id)?;
        if reread_commit != commit {
            return Err(RepositoryError::Corruption);
        }

        let announcement_id = announcement_storage_id(&publication.announcement);
        let announcement_bytes = ObjectBytes::new(publication.announcement.clone())
            .map_err(|_| RepositoryError::BoundExceeded)?;
        self.store
            .put_immutable(
                &self.address.announcement_bucket,
                &announcement_id,
                &announcement_bytes,
            )
            .map_err(map_store_error)?;
        let reread = self
            .store
            .get(&self.address.announcement_bucket, &announcement_id)
            .map_err(map_store_error)?
            .ok_or(RepositoryError::ProviderWithholding)?;
        if reread.as_slice() != publication.announcement
            || announcement_storage_id(reread.as_slice()) != announcement_id
        {
            return Err(RepositoryError::Corruption);
        }
        let reread_announcement = self.verify_announcement_bytes(reread.as_slice())?;
        Self::cross_check(&commit_id, &reread_commit, &reread_announcement)?;

        Ok(PublicationReceipt {
            commit_id,
            heads: next_heads,
        })
    }

    /// Completely discover and verify the visible repository against local pins.
    pub fn open(&self, pins: &PinnedHeads) -> Result<OpenReport, RepositoryError> {
        self.ensure_initialized()?;
        let announcement_ids =
            self.list_all(&self.address.announcement_bucket, MAX_ANNOUNCEMENTS)?;
        if announcement_ids.is_empty() {
            if pins.is_empty() {
                return Ok(OpenReport {
                    heads: PinnedHeads::empty(),
                    announcement_count: 0,
                    commit_count: 0,
                    fresh_device_unanchored: true,
                });
            }
            return Err(RepositoryError::ProviderWithholding);
        }

        let mut announcements = BTreeMap::new();
        for storage_id in &announcement_ids {
            let bytes = self
                .store
                .get(&self.address.announcement_bucket, storage_id)
                .map_err(map_store_error)?
                .ok_or(RepositoryError::ProviderWithholding)?;
            if announcement_storage_id(bytes.as_slice()) != *storage_id {
                return Err(RepositoryError::Corruption);
            }
            let announcement = self.verify_announcement_bytes(bytes.as_slice())?;
            let prior = announcements.insert(announcement.commit_id, announcement.clone());
            if prior.as_ref().is_some_and(|prior| prior != &announcement) {
                return Err(RepositoryError::Corruption);
            }
        }

        let graph = self.load_graph(announcements.keys().copied())?;
        for (id, announcement) in &announcements {
            let commit = graph.commits.get(id).ok_or(RepositoryError::Corruption)?;
            Self::cross_check(id, commit, announcement)?;
        }
        let heads = graph_heads(&graph, announcements.keys().copied())?;
        for pin in pins.iter() {
            if !graph.commits.contains_key(pin)
                || !heads.iter().any(|head| is_ancestor(&graph, *pin, *head))
            {
                return Err(RepositoryError::ProviderWithholding);
            }
        }

        Ok(OpenReport {
            heads,
            announcement_count: announcement_ids.len(),
            commit_count: graph.commits.len(),
            fresh_device_unanchored: pins.is_empty(),
        })
    }

    /// Explicitly accept verified open heads after any host trust ceremony.
    pub fn accept_open_heads(&self, report: &OpenReport) -> PinnedHeads {
        report.heads.clone()
    }

    /// Read one exact hash-verified encrypted repository object.
    pub fn read_object(&self, id: FormatObjectId) -> Result<VerifiedObject, RepositoryError> {
        self.ensure_initialized()?;
        Ok(VerifiedObject {
            id,
            frame: self.fetch_frame(&id)?,
        })
    }

    /// Read one commit only after complete ancestry and reference verification.
    pub fn read_commit(&self, id: FormatObjectId) -> Result<CommitSummary, RepositoryError> {
        self.ensure_initialized()?;
        let graph = self.load_graph([id])?;
        let commit = graph.commits.get(&id).ok_or(RepositoryError::Corruption)?;
        Ok(CommitSummary::from_commit(id, commit))
    }

    /// Return deterministic verified ancestry beginning with `start`.
    pub fn history(
        &self,
        start: FormatObjectId,
        limit: usize,
    ) -> Result<Vec<CommitSummary>, RepositoryError> {
        self.ensure_initialized()?;
        if limit == 0 || limit > MAX_HISTORY_ENTRIES {
            return Err(RepositoryError::InvalidInput);
        }
        let graph = self.load_graph([start])?;
        let mut order = Vec::new();
        let mut frontier = BTreeSet::from([start]);
        let mut visited = BTreeSet::new();
        while let Some(id) = frontier.pop_first() {
            if !visited.insert(id) {
                continue;
            }
            let commit = graph.commits.get(&id).ok_or(RepositoryError::Corruption)?;
            if order.len() < limit {
                order.push(CommitSummary::from_commit(id, commit));
            }
            frontier.extend(commit.parents.iter().copied());
        }
        Ok(order)
    }

    /// Return complete deterministic verified ancestry beginning with `start`.
    ///
    /// Unlike [`Self::history`], this security-oriented traversal is not
    /// truncated at the interactive history limit. It remains bounded by
    /// [`MAX_GRAPH_COMMITS`] while loading and validating the complete graph.
    pub fn complete_history(
        &self,
        start: FormatObjectId,
    ) -> Result<Vec<CommitSummary>, RepositoryError> {
        self.ensure_initialized()?;
        let graph = self.load_graph([start])?;
        let mut order = Vec::with_capacity(graph.commits.len());
        let mut frontier = BTreeSet::from([start]);
        let mut visited = BTreeSet::new();
        while let Some(id) = frontier.pop_first() {
            if !visited.insert(id) {
                continue;
            }
            let commit = graph.commits.get(&id).ok_or(RepositoryError::Corruption)?;
            order.push(CommitSummary::from_commit(id, commit));
            frontier.extend(commit.parents.iter().copied());
        }
        Ok(order)
    }

    /// Build a complete conservative reachability plan without deleting bytes.
    pub fn plan_gc(&self, retained_heads: &PinnedHeads) -> Result<GcPlan, RepositoryError> {
        self.ensure_initialized()?;
        if retained_heads.is_empty() {
            return Err(RepositoryError::PinConflict);
        }
        let graph = self.load_graph(retained_heads.iter().copied())?;
        let mut reachable = BTreeSet::new();
        for (id, commit) in &graph.commits {
            reachable.insert(*id);
            reachable.extend(referenced_objects(commit));
        }
        let listed = self.list_all(&self.address.object_bucket, MAX_GC_OBJECTS)?;
        let listed_format = listed
            .iter()
            .copied()
            .map(FormatObjectId::from)
            .collect::<BTreeSet<_>>();
        let unreachable = listed_format
            .difference(&reachable)
            .copied()
            .collect::<BTreeSet<_>>();
        Ok(GcPlan {
            listed: listed.len(),
            reachable,
            unreachable,
        })
    }

    fn ensure_initialized(&self) -> Result<(), RepositoryError> {
        if *self
            .initialized
            .lock()
            .map_err(|_| RepositoryError::Storage)?
        {
            Ok(())
        } else {
            Err(RepositoryError::NotInitialized)
        }
    }

    fn verify_announcement_bytes(&self, bytes: &[u8]) -> Result<AnnouncementV1, RepositoryError> {
        let decoded = AnnouncementV1::decode(bytes).map_err(|_| RepositoryError::Corruption)?;
        let verified = self
            .verifier
            .verify_announcement(bytes)
            .map_err(|_| RepositoryError::Verification)?;
        if decoded != verified {
            return Err(RepositoryError::Corruption);
        }
        Ok(decoded)
    }

    fn verify_commit_frame(
        &self,
        expected: &FormatObjectId,
        frame: &ObjectFrameV1,
    ) -> Result<CommitV1, RepositoryError> {
        if frame.id().map_err(|_| RepositoryError::Corruption)? != *expected {
            return Err(RepositoryError::Corruption);
        }
        let commit = self
            .verifier
            .verify_commit(expected, frame)
            .map_err(|_| RepositoryError::Verification)?;
        commit.validate().map_err(|_| RepositoryError::Corruption)?;
        Ok(commit)
    }

    fn fetch_frame(&self, id: &FormatObjectId) -> Result<ObjectFrameV1, RepositoryError> {
        let storage_id = StorageObjectId::from(*id);
        let bytes = self
            .store
            .get(&self.address.object_bucket, &storage_id)
            .map_err(map_store_error)?
            .ok_or(RepositoryError::ProviderWithholding)?;
        let frame =
            ObjectFrameV1::decode(bytes.as_slice()).map_err(|_| RepositoryError::Corruption)?;
        if frame.id().map_err(|_| RepositoryError::Corruption)? != *id {
            return Err(RepositoryError::Corruption);
        }
        Ok(frame)
    }

    fn fetch_commit(&self, id: &FormatObjectId) -> Result<CommitV1, RepositoryError> {
        let frame = self.fetch_frame(id)?;
        self.verify_commit_frame(id, &frame)
    }

    fn put_frame_exact(&self, id: &FormatObjectId, encoded: &[u8]) -> Result<(), RepositoryError> {
        let storage_id = StorageObjectId::from(*id);
        let bytes =
            ObjectBytes::new(encoded.to_vec()).map_err(|_| RepositoryError::BoundExceeded)?;
        self.store
            .put_immutable(&self.address.object_bucket, &storage_id, &bytes)
            .map_err(map_store_error)?;
        let reread = self
            .store
            .get(&self.address.object_bucket, &storage_id)
            .map_err(map_store_error)?
            .ok_or(RepositoryError::ProviderWithholding)?;
        if reread.as_slice() != encoded {
            return Err(RepositoryError::Corruption);
        }
        let frame =
            ObjectFrameV1::decode(reread.as_slice()).map_err(|_| RepositoryError::Corruption)?;
        if frame.id().map_err(|_| RepositoryError::Corruption)? != *id {
            return Err(RepositoryError::Corruption);
        }
        Ok(())
    }

    fn load_graph(
        &self,
        roots: impl IntoIterator<Item = FormatObjectId>,
    ) -> Result<VerifiedGraph, RepositoryError> {
        let mut frontier = roots.into_iter().collect::<BTreeSet<_>>();
        let mut commits = BTreeMap::new();
        while let Some(id) = frontier.pop_first() {
            if commits.contains_key(&id) {
                continue;
            }
            if commits.len() >= MAX_GRAPH_COMMITS {
                return Err(RepositoryError::BoundExceeded);
            }
            let commit = self.fetch_commit(&id)?;
            for referenced in referenced_objects(&commit) {
                self.fetch_frame(&referenced)?;
            }
            frontier.extend(commit.parents.iter().copied());
            commits.insert(id, commit);
        }
        let graph = VerifiedGraph { commits };
        validate_graph(&graph)?;
        Ok(graph)
    }

    fn list_all(
        &self,
        bucket: &BucketId,
        bound: usize,
    ) -> Result<Vec<StorageObjectId>, RepositoryError> {
        let mut cursor: Option<ListCursor> = None;
        let mut ids = Vec::new();
        let mut previous = None;
        loop {
            let prior_cursor = cursor.as_ref().map(|value| value.as_bytes().to_vec());
            let page = self
                .store
                .list(bucket, cursor.as_ref(), REPOSITORY_LIST_PAGE_SIZE)
                .map_err(map_store_error)?;
            for entry in page.entries {
                if previous.is_some_and(|prior| prior >= entry.object) {
                    return Err(RepositoryError::Corruption);
                }
                if ids.len() >= bound {
                    return Err(RepositoryError::BoundExceeded);
                }
                previous = Some(entry.object);
                ids.push(entry.object);
            }
            let Some(next) = page.next_cursor else {
                break;
            };
            if ids.is_empty()
                || prior_cursor
                    .as_ref()
                    .is_some_and(|prior| prior.as_slice() == next.as_bytes())
            {
                return Err(RepositoryError::Corruption);
            }
            cursor = Some(next);
        }
        Ok(ids)
    }

    fn cross_check(
        commit_id: &FormatObjectId,
        commit: &CommitV1,
        announcement: &AnnouncementV1,
    ) -> Result<(), RepositoryError> {
        if announcement.commit_id != *commit_id
            || announcement.vault_id != commit.vault_id
            || announcement.device_id != commit.device_id
            || announcement.device_counter != commit.device_counter
            || announcement.device_certificate != commit.device_certificate
        {
            return Err(RepositoryError::Corruption);
        }
        Ok(())
    }
}

impl<S: VaultObjectStore, V: RepositoryVerifier> Debug for Repository<S, V> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let initialized = self.initialized.lock().map(|value| *value).unwrap_or(false);
        formatter
            .debug_struct("Repository")
            .field("store", &"<redacted>")
            .field("verifier", &"<redacted>")
            .field("address", &"<redacted>")
            .field("initialized", &initialized)
            .finish()
    }
}

fn referenced_objects(commit: &CommitV1) -> BTreeSet<FormatObjectId> {
    let mut referenced = commit
        .added_objects
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    referenced.insert(commit.catalog_root);
    referenced.insert(commit.device_certificate);
    if let Some(tombstone) = commit.tombstone_root {
        referenced.insert(tombstone);
    }
    referenced
}

fn validate_graph(graph: &VerifiedGraph) -> Result<(), RepositoryError> {
    let mut counters = BTreeMap::new();
    let mut remaining_parents = BTreeMap::new();
    let mut children: BTreeMap<FormatObjectId, Vec<FormatObjectId>> = BTreeMap::new();
    for (id, commit) in &graph.commits {
        if let Some(prior) = counters.insert((commit.device_id, commit.device_counter), *id) {
            if prior != *id {
                return Err(RepositoryError::DeviceEquivocation);
            }
        }
        let parent_count = commit
            .parents
            .iter()
            .filter(|parent| graph.commits.contains_key(parent))
            .count();
        remaining_parents.insert(*id, parent_count);
        for parent in &commit.parents {
            children.entry(*parent).or_default().push(*id);
        }
    }

    let mut ready = remaining_parents
        .iter()
        .filter_map(|(id, count)| (*count == 0).then_some(*id))
        .collect::<VecDeque<_>>();
    let mut visited = 0;
    let mut topological = Vec::with_capacity(graph.commits.len());
    while let Some(id) = ready.pop_front() {
        visited += 1;
        topological.push(id);
        if let Some(children) = children.get(&id) {
            for child in children {
                let count = remaining_parents
                    .get_mut(child)
                    .ok_or(RepositoryError::Corruption)?;
                *count = count.checked_sub(1).ok_or(RepositoryError::Corruption)?;
                if *count == 0 {
                    ready.push_back(*child);
                }
            }
        }
    }
    if visited != graph.commits.len() {
        return Err(RepositoryError::GraphCycle);
    }
    validate_counter_ancestry(graph, &topological)?;
    Ok(())
}

fn validate_counter_ancestry(
    graph: &VerifiedGraph,
    topological: &[FormatObjectId],
) -> Result<(), RepositoryError> {
    for id in topological {
        let current = graph.commits.get(id).ok_or(RepositoryError::Corruption)?;
        let mut frontier = current.parents.clone();
        let mut visited = BTreeSet::new();
        while let Some(ancestor_id) = frontier.pop() {
            if !visited.insert(ancestor_id) {
                continue;
            }
            let ancestor = graph
                .commits
                .get(&ancestor_id)
                .ok_or(RepositoryError::Corruption)?;
            if ancestor.device_id == current.device_id {
                if ancestor.device_counter >= current.device_counter {
                    return Err(RepositoryError::DeviceEquivocation);
                }
                // This ancestor was validated earlier in topological order, so
                // its same-device ancestry is already strictly lower.
                continue;
            }
            frontier.extend(ancestor.parents.iter().copied());
        }
    }
    Ok(())
}

fn graph_heads(
    graph: &VerifiedGraph,
    announced: impl IntoIterator<Item = FormatObjectId>,
) -> Result<PinnedHeads, RepositoryError> {
    let parents = graph
        .commits
        .values()
        .flat_map(|commit| commit.parents.iter().copied())
        .collect::<BTreeSet<_>>();
    PinnedHeads::new(announced.into_iter().filter(|id| !parents.contains(id)))
}

fn is_ancestor(graph: &VerifiedGraph, ancestor: FormatObjectId, head: FormatObjectId) -> bool {
    let mut frontier = vec![head];
    let mut visited = BTreeSet::new();
    while let Some(id) = frontier.pop() {
        if id == ancestor {
            return true;
        }
        if visited.insert(id) {
            if let Some(commit) = graph.commits.get(&id) {
                frontier.extend(commit.parents.iter().copied());
            }
        }
    }
    false
}

fn map_store_error(error: StoreError) -> RepositoryError {
    match error {
        StoreError::Corruption => RepositoryError::Corruption,
        StoreError::NotInitialized => RepositoryError::NotInitialized,
        _ => RepositoryError::Storage,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_pm_format::Signature;
    use coding_adventures_vault_pm_storage::{
        BackendCapabilities, ChangeCursor, ChangePage, DeleteOutcome, FaultAction, FaultEffect,
        FaultInjectingObjectStore, InMemoryObjectStore, ObjectPage, ObjectStat,
        PutImmutableOutcome, StoreOperation,
    };
    use std::sync::Arc;

    const VALID_SIGNATURE: Signature = Signature::new([9; 64]);

    #[derive(Clone)]
    struct SharedStore(Arc<InMemoryObjectStore>);

    impl SharedStore {
        fn new() -> Self {
            Self(Arc::new(InMemoryObjectStore::new()))
        }
    }

    impl VaultObjectStore for SharedStore {
        fn initialize(&self, locator: &VaultLocator) -> Result<(), StoreError> {
            self.0.initialize(locator)
        }

        fn capabilities(&self) -> BackendCapabilities {
            self.0.capabilities()
        }

        fn get(
            &self,
            bucket: &BucketId,
            object: &StorageObjectId,
        ) -> Result<Option<ObjectBytes>, StoreError> {
            self.0.get(bucket, object)
        }

        fn stat(
            &self,
            bucket: &BucketId,
            object: &StorageObjectId,
        ) -> Result<Option<ObjectStat>, StoreError> {
            self.0.stat(bucket, object)
        }

        fn put_immutable(
            &self,
            bucket: &BucketId,
            object: &StorageObjectId,
            bytes: &ObjectBytes,
        ) -> Result<PutImmutableOutcome, StoreError> {
            self.0.put_immutable(bucket, object, bytes)
        }

        fn list(
            &self,
            bucket: &BucketId,
            cursor: Option<&ListCursor>,
            limit: usize,
        ) -> Result<ObjectPage, StoreError> {
            self.0.list(bucket, cursor, limit)
        }

        fn delete_unreferenced(
            &self,
            bucket: &BucketId,
            object: &StorageObjectId,
        ) -> Result<DeleteOutcome, StoreError> {
            self.0.delete_unreferenced(bucket, object)
        }

        fn changes(&self, cursor: Option<&ChangeCursor>) -> Result<Option<ChangePage>, StoreError> {
            self.0.changes(cursor)
        }
    }

    struct FixtureVerifier;

    impl RepositoryVerifier for FixtureVerifier {
        fn verify_commit(
            &self,
            _expected: &FormatObjectId,
            frame: &ObjectFrameV1,
        ) -> Result<CommitV1, VerificationError> {
            let commit = CommitV1::decode(&frame.ciphertext).map_err(|_| VerificationError)?;
            if commit.signature != VALID_SIGNATURE {
                return Err(VerificationError);
            }
            Ok(commit)
        }

        fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError> {
            let announcement = AnnouncementV1::decode(bytes).map_err(|_| VerificationError)?;
            if announcement.signature != VALID_SIGNATURE {
                return Err(VerificationError);
            }
            Ok(announcement)
        }
    }

    struct MapVerifier {
        commits: BTreeMap<FormatObjectId, CommitV1>,
    }

    impl RepositoryVerifier for MapVerifier {
        fn verify_commit(
            &self,
            expected: &FormatObjectId,
            _frame: &ObjectFrameV1,
        ) -> Result<CommitV1, VerificationError> {
            self.commits.get(expected).cloned().ok_or(VerificationError)
        }

        fn verify_announcement(&self, bytes: &[u8]) -> Result<AnnouncementV1, VerificationError> {
            FixtureVerifier.verify_announcement(bytes)
        }
    }

    fn address() -> RepositoryAddress {
        RepositoryAddress::derive(&[7; 32])
    }

    fn frame(ciphertext: Vec<u8>) -> ObjectFrameV1 {
        ObjectFrameV1 {
            suite: 1,
            wrap_nonce: [1; 24],
            wrapped_dek: [2; 32],
            wrap_tag: [3; 16],
            payload_nonce: [4; 24],
            ciphertext,
            payload_tag: [5; 16],
        }
    }

    fn sorted(mut ids: Vec<FormatObjectId>) -> Vec<FormatObjectId> {
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn signed_commit(
        parents: Vec<FormatObjectId>,
        catalog_root: FormatObjectId,
        certificate: FormatObjectId,
        added_objects: Vec<FormatObjectId>,
        device_byte: u8,
        counter: u64,
    ) -> (ObjectFrameV1, FormatObjectId, CommitV1) {
        let commit = CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([device_byte; 16]),
            device_counter: counter,
            parents: sorted(parents),
            catalog_root,
            added_objects: sorted(added_objects),
            tombstone_root: None,
            wall_time_ms: counter * 1_000,
            device_certificate: certificate,
            signature: VALID_SIGNATURE,
        };
        let commit_frame = frame(commit.encode().unwrap());
        let id = commit_frame.id().unwrap();
        (commit_frame, id, commit)
    }

    fn signed_announcement(commit_id: FormatObjectId, commit: &CommitV1) -> Vec<u8> {
        AnnouncementV1 {
            vault_id: commit.vault_id,
            device_id: commit.device_id,
            device_counter: commit.device_counter,
            commit_id,
            device_certificate: commit.device_certificate,
            signature: VALID_SIGNATURE,
        }
        .encode()
        .unwrap()
    }

    fn make_repository(store: SharedStore) -> Repository<SharedStore, FixtureVerifier> {
        let repository = Repository::new(store, FixtureVerifier, address());
        repository.initialize().unwrap();
        repository
    }

    fn put_frame<S: VaultObjectStore>(store: &S, value: &ObjectFrameV1) -> FormatObjectId {
        let id = value.id().unwrap();
        store
            .put_immutable(
                &address().object_bucket(),
                &StorageObjectId::from(id),
                &ObjectBytes::new(value.encode().unwrap()).unwrap(),
            )
            .unwrap();
        id
    }

    fn put_announcement<S: VaultObjectStore>(store: &S, bytes: &[u8]) {
        store
            .put_immutable(
                &address().announcement_bucket(),
                &announcement_storage_id(bytes),
                &ObjectBytes::new(bytes.to_vec()).unwrap(),
            )
            .unwrap();
    }

    fn genesis_publication() -> (
        Publication,
        ObjectFrameV1,
        ObjectFrameV1,
        ObjectFrameV1,
        FormatObjectId,
        CommitV1,
    ) {
        let catalog = frame(b"catalog-1".to_vec());
        let certificate = frame(b"certificate".to_vec());
        let catalog_id = catalog.id().unwrap();
        let certificate_id = certificate.id().unwrap();
        let (commit_frame, commit_id, commit) = signed_commit(
            vec![],
            catalog_id,
            certificate_id,
            vec![catalog_id, certificate_id],
            1,
            1,
        );
        let publication = Publication::new(
            vec![catalog.clone(), certificate.clone()],
            commit_frame.clone(),
            signed_announcement(commit_id, &commit),
        );
        (
            publication,
            catalog,
            certificate,
            commit_frame,
            commit_id,
            commit,
        )
    }

    fn decode_hex_32(value: &str) -> [u8; 32] {
        assert_eq!(value.len(), 64);
        let mut bytes = [0; 32];
        for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let text = std::str::from_utf8(pair).unwrap();
            bytes[index] = u8::from_str_radix(text, 16).unwrap();
        }
        bytes
    }

    #[test]
    fn address_and_announcement_id_match_vectors() {
        let address = address();
        assert_eq!(
            address.store_locator().as_bytes(),
            &decode_hex_32("a2f2913e6e8dcefb3309f38610b10e0a98dc63cd09dc0fe94331222f68557a1a")
        );
        assert_eq!(
            address.object_bucket().as_bytes(),
            &decode_hex_32("9fcd6e85cef61f4fbd728a3612c1b79f0eb5a5e9a9af60e726d4dc25333446f4")
        );
        assert_eq!(
            address.announcement_bucket().as_bytes(),
            &decode_hex_32("1440812a20b4e9685213d1fe15fabb6bf98f3a54c5a7209bed465073bda2e937")
        );
        assert_eq!(
            announcement_storage_id(&[1, 2, 3]).as_bytes(),
            &decode_hex_32("0365b358b283e3e6557a41ddd0103b8c2c505270809c9483986db8af05d24ce5")
        );
    }

    #[test]
    fn operations_fail_closed_before_initialization() {
        let repository = Repository::new(SharedStore::new(), FixtureVerifier, address());
        let (publication, ..) = genesis_publication();
        assert_eq!(
            repository.publish(publication, &PinnedHeads::empty()),
            Err(RepositoryError::NotInitialized)
        );
        assert_eq!(
            repository.open(&PinnedHeads::empty()),
            Err(RepositoryError::NotInitialized)
        );
        assert_eq!(
            repository.complete_history(FormatObjectId::new([1; 32])),
            Err(RepositoryError::NotInitialized)
        );
        assert_eq!(
            repository.plan_gc(&PinnedHeads::empty()),
            Err(RepositoryError::NotInitialized)
        );
    }

    #[test]
    fn publication_open_retry_and_restart_are_idempotent() {
        let store = SharedStore::new();
        let repository = make_repository(store.clone());
        let (publication, catalog, certificate, commit_frame, commit_id, commit) =
            genesis_publication();
        let catalog_id = catalog.id().unwrap();
        let certificate_id = certificate.id().unwrap();
        let receipt = repository
            .publish(publication, &PinnedHeads::empty())
            .unwrap();
        assert_eq!(receipt.commit_id(), commit_id);
        assert_eq!(receipt.heads().len(), 1);
        assert!(format!("{receipt:?}").contains("head_count: 1"));
        let verified_object = repository.read_object(catalog_id).unwrap();
        assert_eq!(verified_object.id(), catalog_id);
        assert_eq!(verified_object.frame(), &catalog);
        assert!(format!("{verified_object:?}").contains("<redacted>"));
        assert_eq!(verified_object.clone().into_frame(), catalog);
        let verified_commit = repository.read_commit(commit_id).unwrap();
        assert_eq!(verified_commit.catalog_root(), catalog_id);
        assert_eq!(verified_commit.device_certificate(), certificate_id);
        assert_eq!(
            verified_commit.added_objects(),
            sorted(vec![catalog_id, certificate_id])
        );
        assert_eq!(verified_commit.tombstone_root(), None);

        let replay = Publication::new(
            vec![catalog, certificate],
            commit_frame,
            signed_announcement(commit_id, &commit),
        );
        assert_eq!(
            repository.publish(replay, &PinnedHeads::empty()).unwrap(),
            receipt
        );
        let report = repository.open(receipt.heads()).unwrap();
        assert_eq!(report.heads(), receipt.heads());
        assert_eq!(report.announcement_count(), 1);
        assert_eq!(report.commit_count(), 1);
        assert!(!report.fresh_device_unanchored());
        assert!(format!("{report:?}").contains("announcement_count: 1"));

        let reopened = make_repository(store);
        let reopened_report = reopened.open(receipt.heads()).unwrap();
        assert_eq!(
            reopened.accept_open_heads(&reopened_report),
            *receipt.heads()
        );
    }

    #[test]
    fn ambiguous_commit_put_is_safe_to_retry() {
        let shared = SharedStore::new();
        shared.initialize(&address().store_locator()).unwrap();
        let (_, catalog, certificate, commit_frame, commit_id, commit) = genesis_publication();
        put_frame(&shared, &catalog);
        put_frame(&shared, &certificate);
        let faulty = FaultInjectingObjectStore::new(shared.clone());
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();
        let repository = Repository::new(faulty, FixtureVerifier, address());
        repository.initialize().unwrap();
        let make_publication = || {
            Publication::new(
                vec![],
                commit_frame.clone(),
                signed_announcement(commit_id, &commit),
            )
        };
        assert_eq!(
            repository.publish(make_publication(), &PinnedHeads::empty()),
            Err(RepositoryError::Storage)
        );
        let receipt = repository
            .publish(make_publication(), &PinnedHeads::empty())
            .unwrap();
        assert_eq!(receipt.commit_id(), commit_id);
        assert_eq!(repository.open(receipt.heads()).unwrap().commit_count(), 1);
    }

    #[test]
    fn publication_rejects_missing_refs_bad_signature_and_cross_fields() {
        let store = SharedStore::new();
        let repository = make_repository(store);
        let (_, catalog, certificate, commit_frame, commit_id, commit) = genesis_publication();
        assert_eq!(
            repository.publish(
                Publication::new(
                    vec![catalog.clone()],
                    commit_frame.clone(),
                    signed_announcement(commit_id, &commit),
                ),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::ProviderWithholding)
        );

        let mut bad_commit = commit.clone();
        bad_commit.signature = Signature::new([8; 64]);
        let bad_frame = frame(bad_commit.encode().unwrap());
        let bad_id = bad_frame.id().unwrap();
        assert_eq!(
            repository.publish(
                Publication::new(
                    vec![catalog.clone(), certificate.clone()],
                    bad_frame,
                    signed_announcement(bad_id, &bad_commit),
                ),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::Verification)
        );

        let mut wrong = AnnouncementV1::decode(&signed_announcement(commit_id, &commit)).unwrap();
        wrong.device_counter += 1;
        let wrong = wrong.with_signature(VALID_SIGNATURE).encode().unwrap();
        assert_eq!(
            repository.publish(
                Publication::new(vec![catalog, certificate], commit_frame, wrong),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::Corruption)
        );
    }

    #[test]
    fn local_pins_detect_provider_withholding() {
        let repository = make_repository(SharedStore::new());
        let report = repository.open(&PinnedHeads::empty()).unwrap();
        assert!(report.fresh_device_unanchored());
        let absent = PinnedHeads::new([FormatObjectId::new([99; 32])]).unwrap();
        assert_eq!(
            repository.open(&absent),
            Err(RepositoryError::ProviderWithholding)
        );
    }

    #[test]
    fn branches_merge_into_deterministic_heads_and_history() {
        let store = SharedStore::new();
        let repository = make_repository(store);
        let (genesis, _, certificate, _, genesis_id, _) = genesis_publication();
        let genesis_receipt = repository.publish(genesis, &PinnedHeads::empty()).unwrap();
        let certificate_id = certificate.id().unwrap();

        let catalog_a = frame(b"catalog-a".to_vec());
        let catalog_a_id = catalog_a.id().unwrap();
        let (frame_a, id_a, commit_a) = signed_commit(
            vec![genesis_id],
            catalog_a_id,
            certificate_id,
            vec![catalog_a_id],
            1,
            2,
        );
        let receipt_a = repository
            .publish(
                Publication::new(
                    vec![catalog_a],
                    frame_a,
                    signed_announcement(id_a, &commit_a),
                ),
                genesis_receipt.heads(),
            )
            .unwrap();

        let catalog_b = frame(b"catalog-b".to_vec());
        let catalog_b_id = catalog_b.id().unwrap();
        let (frame_b, id_b, commit_b) = signed_commit(
            vec![genesis_id],
            catalog_b_id,
            certificate_id,
            vec![catalog_b_id],
            2,
            1,
        );
        let receipt_b = repository
            .publish(
                Publication::new(
                    vec![catalog_b],
                    frame_b,
                    signed_announcement(id_b, &commit_b),
                ),
                receipt_a.heads(),
            )
            .unwrap();
        assert_eq!(receipt_b.heads().len(), 2);

        let catalog_merge = frame(b"catalog-merge".to_vec());
        let catalog_merge_id = catalog_merge.id().unwrap();
        let (merge_frame, merge_id, merge_commit) = signed_commit(
            vec![id_a, id_b],
            catalog_merge_id,
            certificate_id,
            vec![catalog_merge_id],
            1,
            3,
        );
        let merge_receipt = repository
            .publish(
                Publication::new(
                    vec![catalog_merge],
                    merge_frame,
                    signed_announcement(merge_id, &merge_commit),
                ),
                receipt_b.heads(),
            )
            .unwrap();
        assert_eq!(merge_receipt.heads().len(), 1);

        let report = repository.open(merge_receipt.heads()).unwrap();
        assert_eq!(report.heads(), merge_receipt.heads());
        assert_eq!(report.commit_count(), 4);
        let history = repository.history(merge_id, 10).unwrap();
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].id(), merge_id);
        assert_eq!(history[0].vault_id(), VaultId::new([1; 16]));
        assert_eq!(history[0].device_id(), DeviceId::new([1; 16]));
        assert_eq!(history[0].device_counter(), 3);
        assert_eq!(history[0].wall_time_ms(), 3_000);
        assert!(history.iter().any(|summary| summary.id() == genesis_id));
        assert_eq!(history[0].parents(), sorted(vec![id_a, id_b]));
        assert!(format!("{:?}", history[0]).contains("parent_count: 2"));

        let complete = repository.complete_history(merge_id).unwrap();
        assert_eq!(complete, history);
    }

    #[test]
    fn gc_plan_retains_history_and_only_reports_orphans() {
        let store = SharedStore::new();
        let repository = make_repository(store.clone());
        let (publication, ..) = genesis_publication();
        let receipt = repository
            .publish(publication, &PinnedHeads::empty())
            .unwrap();
        let orphan = frame(b"orphan".to_vec());
        let orphan_id = put_frame(&store, &orphan);
        let plan = repository.plan_gc(receipt.heads()).unwrap();
        assert!(plan.listed_count() >= 4);
        assert!(plan.unreachable().contains(&orphan_id));
        assert!(!plan.reachable().contains(&orphan_id));
        assert_eq!(plan.unreachable().len(), 1);
        assert!(format!("{plan:?}").contains("unreachable_count: 1"));
        assert_eq!(
            repository.plan_gc(&PinnedHeads::empty()),
            Err(RepositoryError::PinConflict)
        );
    }

    #[test]
    fn corrupted_provider_read_is_never_accepted() {
        let shared = SharedStore::new();
        let normal = make_repository(shared.clone());
        let (publication, ..) = genesis_publication();
        let receipt = normal.publish(publication, &PinnedHeads::empty()).unwrap();
        drop(normal);

        let faulty = FaultInjectingObjectStore::new(shared);
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::Get,
                effect: FaultEffect::CorruptGet,
            })
            .unwrap();
        let repository = Repository::new(faulty, FixtureVerifier, address());
        repository.initialize().unwrap();
        assert_eq!(
            repository.open(receipt.heads()),
            Err(RepositoryError::Corruption)
        );
    }

    #[test]
    fn malformed_duplicate_listing_is_corruption() {
        let shared = SharedStore::new();
        let normal = make_repository(shared.clone());
        let (publication, ..) = genesis_publication();
        normal.publish(publication, &PinnedHeads::empty()).unwrap();
        drop(normal);

        let faulty = FaultInjectingObjectStore::new(shared);
        faulty
            .enqueue(FaultAction {
                operation: StoreOperation::List,
                effect: FaultEffect::DuplicateFirstListEntry,
            })
            .unwrap();
        let repository = Repository::new(faulty, FixtureVerifier, address());
        repository.initialize().unwrap();
        assert_eq!(
            repository.open(&PinnedHeads::empty()),
            Err(RepositoryError::Corruption)
        );
    }

    fn manually_seed_graph(
        store: &SharedStore,
        entries: &[(ObjectFrameV1, FormatObjectId, CommitV1)],
        catalog: &ObjectFrameV1,
        certificate: &ObjectFrameV1,
    ) {
        store.initialize(&address().store_locator()).unwrap();
        put_frame(store, catalog);
        put_frame(store, certificate);
        for (commit_frame, commit_id, commit) in entries {
            assert_eq!(put_frame(store, commit_frame), *commit_id);
            put_announcement(store, &signed_announcement(*commit_id, commit));
        }
    }

    #[test]
    fn device_counter_equivocation_is_detected() {
        let store = SharedStore::new();
        let catalog = frame(b"catalog".to_vec());
        let certificate = frame(b"certificate".to_vec());
        let catalog_id = catalog.id().unwrap();
        let certificate_id = certificate.id().unwrap();
        let frame_a = frame(b"opaque-a".to_vec());
        let frame_b = frame(b"opaque-b".to_vec());
        let id_a = frame_a.id().unwrap();
        let id_b = frame_b.id().unwrap();
        let make = || CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([1; 16]),
            device_counter: 1,
            parents: vec![],
            catalog_root: catalog_id,
            added_objects: sorted(vec![catalog_id, certificate_id]),
            tombstone_root: None,
            wall_time_ms: 1,
            device_certificate: certificate_id,
            signature: VALID_SIGNATURE,
        };
        let commit_a = make();
        let commit_b = make();
        manually_seed_graph(
            &store,
            &[
                (frame_a, id_a, commit_a.clone()),
                (frame_b, id_b, commit_b.clone()),
            ],
            &catalog,
            &certificate,
        );
        let verifier = MapVerifier {
            commits: BTreeMap::from([(id_a, commit_a), (id_b, commit_b)]),
        };
        let repository = Repository::new(store, verifier, address());
        repository.initialize().unwrap();
        assert_eq!(
            repository.open(&PinnedHeads::empty()),
            Err(RepositoryError::DeviceEquivocation)
        );
    }

    #[test]
    fn graph_cycles_are_detected_iteratively() {
        let store = SharedStore::new();
        let catalog = frame(b"catalog".to_vec());
        let certificate = frame(b"certificate".to_vec());
        let catalog_id = catalog.id().unwrap();
        let certificate_id = certificate.id().unwrap();
        let frame_a = frame(b"opaque-cycle-a".to_vec());
        let frame_b = frame(b"opaque-cycle-b".to_vec());
        let id_a = frame_a.id().unwrap();
        let id_b = frame_b.id().unwrap();
        let make = |device_byte, counter, parent| CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([device_byte; 16]),
            device_counter: counter,
            parents: vec![parent],
            catalog_root: catalog_id,
            added_objects: sorted(vec![catalog_id, certificate_id]),
            tombstone_root: None,
            wall_time_ms: counter,
            device_certificate: certificate_id,
            signature: VALID_SIGNATURE,
        };
        let commit_a = make(1, 1, id_b);
        let commit_b = make(2, 1, id_a);
        manually_seed_graph(
            &store,
            &[
                (frame_a, id_a, commit_a.clone()),
                (frame_b, id_b, commit_b.clone()),
            ],
            &catalog,
            &certificate,
        );
        let repository = Repository::new(
            store,
            MapVerifier {
                commits: BTreeMap::from([(id_a, commit_a), (id_b, commit_b)]),
            },
            address(),
        );
        repository.initialize().unwrap();
        assert_eq!(
            repository.open(&PinnedHeads::empty()),
            Err(RepositoryError::GraphCycle)
        );
    }

    #[test]
    fn same_device_counter_regression_through_another_device_is_detected() {
        let store = SharedStore::new();
        let catalog = frame(b"catalog".to_vec());
        let certificate = frame(b"certificate".to_vec());
        let catalog_id = catalog.id().unwrap();
        let certificate_id = certificate.id().unwrap();
        let oldest_frame = frame(b"counter-oldest".to_vec());
        let middle_frame = frame(b"counter-middle".to_vec());
        let newest_frame = frame(b"counter-newest".to_vec());
        let oldest_id = oldest_frame.id().unwrap();
        let middle_id = middle_frame.id().unwrap();
        let newest_id = newest_frame.id().unwrap();
        let make = |device_byte, counter, parents| CommitV1 {
            vault_id: VaultId::new([1; 16]),
            device_id: DeviceId::new([device_byte; 16]),
            device_counter: counter,
            parents,
            catalog_root: catalog_id,
            added_objects: sorted(vec![catalog_id, certificate_id]),
            tombstone_root: None,
            wall_time_ms: counter,
            device_certificate: certificate_id,
            signature: VALID_SIGNATURE,
        };
        let oldest = make(1, 2, vec![]);
        let middle = make(2, 1, vec![oldest_id]);
        let newest = make(1, 1, vec![middle_id]);
        manually_seed_graph(
            &store,
            &[
                (oldest_frame, oldest_id, oldest.clone()),
                (middle_frame, middle_id, middle.clone()),
                (newest_frame, newest_id, newest.clone()),
            ],
            &catalog,
            &certificate,
        );
        let repository = Repository::new(
            store,
            MapVerifier {
                commits: BTreeMap::from([
                    (oldest_id, oldest),
                    (middle_id, middle),
                    (newest_id, newest),
                ]),
            },
            address(),
        );
        repository.initialize().unwrap();
        assert_eq!(
            repository.open(&PinnedHeads::empty()),
            Err(RepositoryError::DeviceEquivocation)
        );
    }

    #[test]
    fn bounds_and_diagnostics_are_closed() {
        let too_many = vec![FormatObjectId::new([1; 32]); MAX_HEAD_PINS + 1];
        assert!(PinnedHeads::new(too_many).is_ok());
        let distinct = (0..=MAX_HEAD_PINS)
            .map(|index| {
                let mut id = [0; 32];
                id[..8].copy_from_slice(&(index as u64).to_be_bytes());
                FormatObjectId::new(id)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            PinnedHeads::new(distinct),
            Err(RepositoryError::BoundExceeded)
        );

        let repository = make_repository(SharedStore::new());
        assert_eq!(
            repository.history(FormatObjectId::new([1; 32]), 0),
            Err(RepositoryError::InvalidInput)
        );
        let excessive = vec![frame(vec![]); MAX_PUBLICATION_OBJECTS + 1];
        let (_, _, _, commit, id, decoded) = genesis_publication();
        assert_eq!(
            repository.publish(
                Publication::new(excessive, commit, signed_announcement(id, &decoded)),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::BoundExceeded)
        );

        let diagnostics = format!(
            "{:?} {:?} {:?} {:?} {:?}",
            address(),
            repository,
            RepositoryError::Corruption,
            PinnedHeads::new([FormatObjectId::new([0xaa; 32])]).unwrap(),
            genesis_publication().0,
        );
        assert!(!diagnostics.contains("170"));
        assert!(!diagnostics.contains("[9, 9"));
        assert!(!diagnostics.contains("catalog"));
        assert!(diagnostics.contains("<redacted>"));

        let errors = [
            RepositoryError::NotInitialized,
            RepositoryError::InvalidInput,
            RepositoryError::BoundExceeded,
            RepositoryError::Storage,
            RepositoryError::Verification,
            RepositoryError::Corruption,
            RepositoryError::ProviderWithholding,
            RepositoryError::DeviceEquivocation,
            RepositoryError::GraphCycle,
            RepositoryError::PinConflict,
        ];
        for error in errors {
            let rendered = format!("{error:?} {error}");
            assert!(rendered.contains(error.label()));
        }
        assert_eq!(
            VerificationError.to_string(),
            "repository verification failed"
        );
    }

    #[test]
    fn defensive_publication_and_graph_branches_fail_closed() {
        let repository = make_repository(SharedStore::new());
        let (_, catalog, certificate, commit_frame, commit_id, commit) = genesis_publication();
        let announcement = signed_announcement(commit_id, &commit);

        assert_eq!(
            repository.publish(
                Publication::new(
                    vec![catalog.clone(), catalog.clone()],
                    commit_frame.clone(),
                    announcement.clone(),
                ),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::InvalidInput)
        );
        assert_eq!(
            repository.publish(
                Publication::new(
                    vec![commit_frame.clone()],
                    commit_frame.clone(),
                    announcement.clone(),
                ),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::InvalidInput)
        );
        assert_eq!(
            repository.publish(
                Publication::new(
                    vec![catalog, certificate, frame(b"unreferenced".to_vec())],
                    commit_frame,
                    announcement,
                ),
                &PinnedHeads::empty(),
            ),
            Err(RepositoryError::InvalidInput)
        );

        let graph = VerifiedGraph {
            commits: BTreeMap::from([(commit_id, commit.clone())]),
        };
        assert!(!is_ancestor(
            &graph,
            FormatObjectId::new([0xff; 32]),
            commit_id,
        ));

        let mut with_tombstone = commit;
        let tombstone = FormatObjectId::new([0xee; 32]);
        with_tombstone.tombstone_root = Some(tombstone);
        assert!(referenced_objects(&with_tombstone).contains(&tombstone));
        let summary = CommitSummary::from_commit(commit_id, &with_tombstone);
        assert_eq!(summary.tombstone_root(), Some(tombstone));
        assert_eq!(
            map_store_error(StoreError::Corruption),
            RepositoryError::Corruption
        );
        assert_eq!(
            map_store_error(StoreError::NotInitialized),
            RepositoryError::NotInitialized
        );
    }
}
