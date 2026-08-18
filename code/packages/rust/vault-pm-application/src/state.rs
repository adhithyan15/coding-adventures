use crate::ApplicationError;
// No panicking `encode` on a production path in this module; the tests
// below import it separately to build fixture bytes known to be in range.
use coding_adventures_canonical_cbor::{decode, try_encode, CborValue};
use coding_adventures_ed25519::{is_valid_public_key, verify};
use coding_adventures_sha256::sha256;
use coding_adventures_vault_pm_format::{
    AeadEnvelopeV1, AnnouncementV1, BootstrapId, BootstrapV1, DeviceId, ObjectFrameV1, ObjectId,
    PublicKey, VaultId, CRYPTO_SUITE_V1,
};
use coding_adventures_vault_pm_repository::{PinnedHeads, Publication, MAX_PUBLICATION_OBJECTS};
use core::fmt::{self, Debug, Display, Formatter};
use std::collections::{BTreeMap, BTreeSet};

const VERSION: u64 = 1;
const PREPARED_INIT: u64 = 1;
const ACTIVE: u64 = 2;
const PENDING_PUBLICATION: u64 = 3;
const MAX_LOCAL_STATE_BYTES: usize = 32 * 1024 * 1024;
const MAX_BOOTSTRAP_BYTES: usize = 1024 * 1024;
const AUTHORITY_FINGERPRINT_DOMAIN: &[u8] = b"VPM-AUTHORITY-FINGERPRINT-v1";

/// Random provider-independent locator for bootstrap and owner-state records.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BootstrapLocator([u8; 32]);

impl BootstrapLocator {
    /// Construct from independently generated random bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the exact random locator bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for BootstrapLocator {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("BootstrapLocator(<redacted>)")
    }
}

/// Domain-separated fingerprint of the pinned vault authority public key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AuthorityFingerprint([u8; 32]);

impl AuthorityFingerprint {
    /// Derive the V1 authority fingerprint from its exact Ed25519 public key.
    pub fn for_public_key(public_key: PublicKey) -> Self {
        let mut preimage = Vec::with_capacity(AUTHORITY_FINGERPRINT_DOMAIN.len() + 32);
        preimage.extend_from_slice(AUTHORITY_FINGERPRINT_DOMAIN);
        preimage.extend_from_slice(public_key.as_bytes());
        Self(sha256(&preimage))
    }

    /// Construct an already-derived fingerprint while decoding owner state.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the exact fingerprint bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for AuthorityFingerprint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthorityFingerprint(<redacted>)")
    }
}

/// Exact already-randomized, already-signed repository publication journal.
#[derive(Clone, PartialEq, Eq)]
pub struct PublicationJournalV1 {
    objects: Vec<ObjectFrameV1>,
    commit: ObjectFrameV1,
    announcement: Vec<u8>,
    base_heads: PinnedHeads,
    expected_heads: PinnedHeads,
    device_counter: u64,
    catalog_root: ObjectId,
    audit_event_head: Option<ObjectId>,
}

impl PublicationJournalV1 {
    /// Validate and retain one exact retryable publication.
    pub fn new(
        objects: Vec<ObjectFrameV1>,
        commit: ObjectFrameV1,
        announcement: Vec<u8>,
        base_heads: PinnedHeads,
        expected_heads: PinnedHeads,
        device_counter: u64,
        catalog_root: ObjectId,
    ) -> Result<Self, ApplicationError> {
        Self::new_inner(
            objects,
            commit,
            announcement,
            base_heads,
            expected_heads,
            device_counter,
            catalog_root,
            None,
        )
    }

    /// Validate one audit-only publication that reuses the active catalog.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_audit_only(
        objects: Vec<ObjectFrameV1>,
        commit: ObjectFrameV1,
        announcement: Vec<u8>,
        base_heads: PinnedHeads,
        expected_heads: PinnedHeads,
        device_counter: u64,
        catalog_root: ObjectId,
        audit_event_head: ObjectId,
    ) -> Result<Self, ApplicationError> {
        Self::new_inner(
            objects,
            commit,
            announcement,
            base_heads,
            expected_heads,
            device_counter,
            catalog_root,
            Some(audit_event_head),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_inner(
        objects: Vec<ObjectFrameV1>,
        commit: ObjectFrameV1,
        announcement: Vec<u8>,
        base_heads: PinnedHeads,
        expected_heads: PinnedHeads,
        device_counter: u64,
        catalog_root: ObjectId,
        audit_event_head: Option<ObjectId>,
    ) -> Result<Self, ApplicationError> {
        let value = Self {
            objects,
            commit,
            announcement,
            base_heads,
            expected_heads,
            device_counter,
            catalog_root,
            audit_event_head,
        };
        value.validate()?;
        Ok(value)
    }

    /// Borrow the exact encrypted non-commit object frames.
    pub fn objects(&self) -> &[ObjectFrameV1] {
        &self.objects
    }

    /// Borrow the exact encrypted commit frame.
    pub const fn commit(&self) -> &ObjectFrameV1 {
        &self.commit
    }

    /// Borrow the exact signed announcement bytes.
    pub fn announcement(&self) -> &[u8] {
        &self.announcement
    }

    /// Borrow the pins that must still precede this publication.
    pub const fn base_heads(&self) -> &PinnedHeads {
        &self.base_heads
    }

    /// Borrow the pins expected from an exact successful retry.
    pub const fn expected_heads(&self) -> &PinnedHeads {
        &self.expected_heads
    }

    /// Return the already-reserved non-zero device counter.
    pub const fn device_counter(&self) -> u64 {
        self.device_counter
    }

    /// Return the resulting encrypted catalog object ID.
    pub const fn catalog_root(&self) -> ObjectId {
        self.catalog_root
    }

    /// Return the encrypted audit event made the new local per-device head.
    pub const fn audit_event_head(&self) -> Option<ObjectId> {
        self.audit_event_head
    }

    /// Bind one newly published encrypted audit event as the next local head.
    pub(crate) fn with_audit_event_head(
        mut self,
        audit_event_head: ObjectId,
    ) -> Result<Self, ApplicationError> {
        self.audit_event_head = Some(audit_event_head);
        self.validate()?;
        Ok(self)
    }

    /// Reconstruct the VLT-PM04 publication without changing any signed bytes.
    pub fn publication(&self) -> Publication {
        Publication::new(
            self.objects.clone(),
            self.commit.clone(),
            self.announcement.clone(),
        )
    }

    fn validate(&self) -> Result<(), ApplicationError> {
        if self.objects.is_empty() || self.objects.len() > MAX_PUBLICATION_OBJECTS {
            return Err(ApplicationError::BoundExceeded);
        }
        if self.device_counter == 0 || self.expected_heads.is_empty() {
            return Err(ApplicationError::InvalidInput);
        }
        let commit_id = self
            .commit
            .id()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if !self.expected_heads.iter().any(|head| *head == commit_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        let announcement = AnnouncementV1::decode(&self.announcement)
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if announcement.commit_id != commit_id || announcement.device_counter != self.device_counter
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let mut object_ids = BTreeSet::new();
        for frame in &self.objects {
            let id = frame.id().map_err(|_| ApplicationError::IntegrityFailure)?;
            if !object_ids.insert(id) {
                return Err(ApplicationError::IntegrityFailure);
            }
        }
        let supplies_catalog = object_ids.contains(&self.catalog_root);
        if (!supplies_catalog && self.audit_event_head.is_none()) || object_ids.contains(&commit_id)
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        if self.audit_event_head.is_some_and(|head| {
            head == self.catalog_root || !object_ids.contains(&head) || head == commit_id
        }) {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(())
    }
}

impl Debug for PublicationJournalV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PublicationJournalV1")
            .field("object_count", &self.objects.len())
            .field("base_head_count", &self.base_heads.len())
            .field("expected_head_count", &self.expected_heads.len())
            .field("device_counter", &self.device_counter)
            .field("advances_audit", &self.audit_event_head.is_some())
            .finish_non_exhaustive()
    }
}

/// Stable owner-private state installed after generation-zero publication.
#[derive(Clone, PartialEq, Eq)]
pub struct ActiveStateV1 {
    bootstrap_locator: BootstrapLocator,
    vault_id: VaultId,
    bootstrap_id: BootstrapId,
    authority_fingerprint: AuthorityFingerprint,
    device_id: DeviceId,
    device_certificate_id: ObjectId,
    device_certificate_frame: ObjectFrameV1,
    local_secret: AeadEnvelopeV1,
    pinned_heads: PinnedHeads,
    last_device_counter: u64,
    catalog_root: ObjectId,
    audit_event_head: Option<ObjectId>,
}

impl ActiveStateV1 {
    /// Validate and construct the complete V1 active owner state.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        bootstrap_locator: BootstrapLocator,
        vault_id: VaultId,
        bootstrap_id: BootstrapId,
        authority_fingerprint: AuthorityFingerprint,
        device_id: DeviceId,
        device_certificate_id: ObjectId,
        device_certificate_frame: ObjectFrameV1,
        local_secret: AeadEnvelopeV1,
        pinned_heads: PinnedHeads,
        last_device_counter: u64,
        catalog_root: ObjectId,
    ) -> Result<Self, ApplicationError> {
        let value = Self {
            bootstrap_locator,
            vault_id,
            bootstrap_id,
            authority_fingerprint,
            device_id,
            device_certificate_id,
            device_certificate_frame,
            local_secret,
            pinned_heads,
            last_device_counter,
            catalog_root,
            audit_event_head: None,
        };
        value.validate()?;
        Ok(value)
    }

    /// Return the random provider-independent lookup locator.
    pub const fn bootstrap_locator(&self) -> BootstrapLocator {
        self.bootstrap_locator
    }

    /// Return the vault binding.
    pub const fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    /// Return the last accepted signed bootstrap ID.
    pub const fn bootstrap_id(&self) -> BootstrapId {
        self.bootstrap_id
    }

    /// Return the pinned authority fingerprint.
    pub const fn authority_fingerprint(&self) -> AuthorityFingerprint {
        self.authority_fingerprint
    }

    /// Return the current certified device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Return the pinned encrypted certificate frame ID.
    pub const fn device_certificate_id(&self) -> ObjectId {
        self.device_certificate_id
    }

    /// Borrow the exact pinned encrypted certificate frame.
    pub const fn device_certificate_frame(&self) -> &ObjectFrameV1 {
        &self.device_certificate_frame
    }

    /// Borrow the encrypted owner/device secret envelope.
    pub const fn local_secret(&self) -> &AeadEnvelopeV1 {
        &self.local_secret
    }

    /// Borrow the last verified repository head pins.
    pub const fn pinned_heads(&self) -> &PinnedHeads {
        &self.pinned_heads
    }

    /// Return the last durably consumed device counter.
    pub const fn last_device_counter(&self) -> u64 {
        self.last_device_counter
    }

    /// Return the currently pinned encrypted catalog root.
    pub const fn catalog_root(&self) -> ObjectId {
        self.catalog_root
    }

    /// Return the last durably activated encrypted event for this device.
    pub const fn audit_event_head(&self) -> Option<ObjectId> {
        self.audit_event_head
    }

    /// Install a verified encrypted event as this device's first audit head.
    pub(crate) fn with_audit_event_head(
        mut self,
        audit_event_head: ObjectId,
    ) -> Result<Self, ApplicationError> {
        self.audit_event_head = Some(audit_event_head);
        self.validate()?;
        Ok(self)
    }

    pub(crate) fn after_publication(
        &self,
        publication: &PublicationJournalV1,
    ) -> Result<Self, ApplicationError> {
        validate_pending(self, publication)?;
        let next_audit_head = match (self.audit_event_head, publication.audit_event_head) {
            (None, next) => next,
            (Some(previous), Some(next)) if previous != next => Some(next),
            (Some(_), _) => return Err(ApplicationError::IntegrityFailure),
        };
        let next = Self::new(
            self.bootstrap_locator,
            self.vault_id,
            self.bootstrap_id,
            self.authority_fingerprint,
            self.device_id,
            self.device_certificate_id,
            self.device_certificate_frame.clone(),
            self.local_secret.clone(),
            publication.expected_heads.clone(),
            publication.device_counter,
            publication.catalog_root,
        )?;
        match next_audit_head {
            Some(head) => next.with_audit_event_head(head),
            None => Ok(next),
        }
    }

    fn validate(&self) -> Result<(), ApplicationError> {
        if self.last_device_counter == 0 || self.pinned_heads.is_empty() {
            return Err(ApplicationError::InvalidInput);
        }
        if self.audit_event_head == Some(self.catalog_root)
            || self.audit_event_head == Some(self.device_certificate_id)
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        self.local_secret
            .validate()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        let certificate_id = self
            .device_certificate_frame
            .id()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if certificate_id != self.device_certificate_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(())
    }
}

impl Debug for ActiveStateV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ActiveStateV1")
            .field("head_count", &self.pinned_heads.len())
            .field("last_device_counter", &self.last_device_counter)
            .field("audit_enabled", &self.audit_event_head.is_some())
            .finish_non_exhaustive()
    }
}

/// Crash-resumable generation-zero journal recorded before any external write.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedInitV1 {
    bootstrap: Vec<u8>,
    intended_active: ActiveStateV1,
    publication: PublicationJournalV1,
}

impl PreparedInitV1 {
    /// Validate and retain exact bootstrap, repository, and final-state bytes.
    pub fn new(
        bootstrap: Vec<u8>,
        intended_active: ActiveStateV1,
        publication: PublicationJournalV1,
    ) -> Result<Self, ApplicationError> {
        let value = Self {
            bootstrap,
            intended_active,
            publication,
        };
        value.validate()?;
        Ok(value)
    }

    /// Borrow the exact authority-signed bootstrap bytes.
    pub fn bootstrap(&self) -> &[u8] {
        &self.bootstrap
    }

    /// Borrow the state installed only after exact publication verification.
    pub const fn intended_active(&self) -> &ActiveStateV1 {
        &self.intended_active
    }

    /// Borrow the exact generation-zero repository publication.
    pub const fn publication(&self) -> &PublicationJournalV1 {
        &self.publication
    }

    fn validate(&self) -> Result<(), ApplicationError> {
        if self.bootstrap.len() > MAX_BOOTSTRAP_BYTES {
            return Err(ApplicationError::BoundExceeded);
        }
        let bootstrap =
            BootstrapV1::decode(&self.bootstrap).map_err(|_| ApplicationError::IntegrityFailure)?;
        let bootstrap_id = bootstrap
            .id()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        let bootstrap_preimage = bootstrap
            .signing_preimage()
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if !is_valid_public_key(bootstrap.authority_public_key.as_bytes())
            || !verify(
                &bootstrap_preimage,
                bootstrap.signature.as_bytes(),
                bootstrap.authority_public_key.as_bytes(),
            )
            || bootstrap.generation != 0
            || bootstrap.previous_bootstrap.is_some()
            || bootstrap.vault_id != self.intended_active.vault_id
            || bootstrap_id != self.intended_active.bootstrap_id
            || AuthorityFingerprint::for_public_key(bootstrap.authority_public_key)
                != self.intended_active.authority_fingerprint
            || !self.publication.base_heads.is_empty()
            || self.publication.expected_heads != self.intended_active.pinned_heads
            || self.publication.device_counter != self.intended_active.last_device_counter
            || self.publication.catalog_root != self.intended_active.catalog_root
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        let announcement = AnnouncementV1::decode(&self.publication.announcement)
            .map_err(|_| ApplicationError::IntegrityFailure)?;
        if announcement.vault_id != self.intended_active.vault_id
            || announcement.device_id != self.intended_active.device_id
            || announcement.device_certificate != self.intended_active.device_certificate_id
            || !self
                .publication
                .objects
                .iter()
                .any(|frame| frame.id().ok() == Some(self.intended_active.device_certificate_id))
        {
            return Err(ApplicationError::IntegrityFailure);
        }
        Ok(())
    }
}

impl Debug for PreparedInitV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedInitV1")
            .field("bootstrap_bytes", &self.bootstrap.len())
            .field("publication", &self.publication)
            .finish_non_exhaustive()
    }
}

/// Complete canonical owner-state machine persisted by a host adapter.
#[derive(Clone, PartialEq, Eq)]
pub enum LocalVaultStateV1 {
    /// Exact generation-zero work prepared before external writes.
    PreparedInit(PreparedInitV1),
    /// Stable verified local pins and encrypted device custody.
    Active(ActiveStateV1),
    /// Stable old state plus exact mutation bytes awaiting publication.
    PendingPublication {
        /// Last stable state, retained until publication succeeds exactly.
        active: ActiveStateV1,
        /// Exact retryable publication and intended new pins/counter/catalog.
        publication: PublicationJournalV1,
    },
}

impl LocalVaultStateV1 {
    /// Construct and cross-check one pending-publication transition.
    pub fn pending_publication(
        active: ActiveStateV1,
        publication: PublicationJournalV1,
    ) -> Result<Self, ApplicationError> {
        validate_pending(&active, &publication)?;
        Ok(Self::PendingPublication {
            active,
            publication,
        })
    }

    /// Encode one exact bounded closed canonical owner-state record.
    pub fn encode(&self) -> Result<Vec<u8>, ApplicationError> {
        self.validate()?;
        let (state, body) = match self {
            Self::PreparedInit(value) => (PREPARED_INIT, encode_prepared(value)?),
            Self::Active(value) => (ACTIVE, encode_active(value)?),
            Self::PendingPublication {
                active,
                publication,
            } => (
                PENDING_PUBLICATION,
                CborValue::Map(vec![
                    field(1, encode_active(active)?),
                    field(2, encode_publication(publication)?),
                ]),
            ),
        };
        // `check_state_bound` below is 32 MiB and is the outer of two
        // bounds; canonical-CBOR independently stops at 1 MiB. The body
        // holds the publication journal, which admits thousands of
        // prepared objects, so the tighter bound is reachable in ordinary
        // use and must be reported rather than aborting the process.
        let encoded = try_encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(state)),
            field(3, body),
        ]))
        .map_err(crate::codec::map_encode_error)?;
        check_state_bound(&encoded)?;
        Ok(encoded)
    }

    /// Strictly decode and cross-check one exact bounded owner-state record.
    pub fn decode(encoded: &[u8]) -> Result<Self, ApplicationError> {
        check_state_bound(encoded)?;
        let mut fields = closed_fields(decode(encoded)?, &[1, 2, 3])?;
        check_version(take_uint(&mut fields, 1)?)?;
        let state = take_uint(&mut fields, 2)?;
        let body = fields
            .remove(&3)
            .ok_or(ApplicationError::IntegrityFailure)?;
        let value = match state {
            PREPARED_INIT => Self::PreparedInit(decode_prepared(body)?),
            ACTIVE => Self::Active(decode_active(body)?),
            PENDING_PUBLICATION => {
                let mut body = closed_fields(body, &[1, 2])?;
                let active = decode_active(take_value(&mut body, 1)?)?;
                let publication = decode_publication(take_value(&mut body, 2)?)?;
                Self::pending_publication(active, publication)?
            }
            _ => return Err(ApplicationError::Unsupported),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), ApplicationError> {
        match self {
            Self::PreparedInit(value) => value.validate(),
            Self::Active(value) => value.validate(),
            Self::PendingPublication {
                active,
                publication,
            } => {
                active.validate()?;
                publication.validate()?;
                validate_pending(active, publication)
            }
        }
    }
}

impl Debug for LocalVaultStateV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreparedInit(_) => formatter.write_str("LocalVaultStateV1::PreparedInit"),
            Self::Active(_) => formatter.write_str("LocalVaultStateV1::Active"),
            Self::PendingPublication { .. } => {
                formatter.write_str("LocalVaultStateV1::PendingPublication")
            }
        }
    }
}

/// Closed provider failure returned by a bootstrap adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootstrapStoreError {
    /// The provider could not complete the operation.
    Unavailable,
    /// The immutable generation or expected predecessor conflicts.
    Conflict,
    /// Provider bytes violate the adapter's exact-value contract.
    Corruption,
}

impl Display for BootstrapStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "bootstrap store unavailable",
            Self::Conflict => "bootstrap store conflict",
            Self::Corruption => "bootstrap store corruption",
        })
    }
}

impl std::error::Error for BootstrapStoreError {}

/// Provider-discoverable exact signed-bootstrap authority injected by a host.
pub trait BootstrapStore: Send + Sync {
    /// Load the latest exact signed bootstrap bytes for one random locator.
    fn load_latest(
        &self,
        locator: BootstrapLocator,
    ) -> Result<Option<Vec<u8>>, BootstrapStoreError>;

    /// Immutably install one generation after the expected predecessor.
    /// Repeating the exact already-installed generation is idempotent success;
    /// the same generation with different bytes is a conflict.
    fn put_generation(
        &self,
        locator: BootstrapLocator,
        expected_previous: Option<BootstrapId>,
        exact_bootstrap: &[u8],
    ) -> Result<(), BootstrapStoreError>;
}

/// Closed host failure returned by an owner-private local-state adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalStateStoreError {
    /// The host could not durably complete the operation.
    Unavailable,
    /// Compare-exchange observed a different exact current value.
    ConcurrentHost,
    /// Host bytes violate the exact-value or durability contract.
    Corruption,
}

impl Display for LocalStateStoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "local state store unavailable",
            Self::ConcurrentHost => "local state compare-exchange conflict",
            Self::Corruption => "local state store corruption",
        })
    }
}

impl std::error::Error for LocalStateStoreError {}

/// Owner-private exact byte store with mandatory atomic compare-exchange.
pub trait LocalStateStore: Send + Sync {
    /// Load the current exact canonical state bytes, if any.
    fn load(&self, locator: BootstrapLocator) -> Result<Option<Vec<u8>>, LocalStateStoreError>;

    /// Atomically replace the exact expected bytes without overwriting a winner.
    fn compare_exchange(
        &self,
        locator: BootstrapLocator,
        expected: Option<&[u8]>,
        replacement: &[u8],
    ) -> Result<(), LocalStateStoreError>;
}

fn encode_prepared(value: &PreparedInitV1) -> Result<CborValue, ApplicationError> {
    Ok(CborValue::Map(vec![
        field(1, bytes(&value.bootstrap)),
        field(2, encode_active(&value.intended_active)?),
        field(3, encode_publication(&value.publication)?),
    ]))
}

fn validate_pending(
    active: &ActiveStateV1,
    publication: &PublicationJournalV1,
) -> Result<(), ApplicationError> {
    if publication.base_heads != active.pinned_heads
        || active.last_device_counter.checked_add(1) != Some(publication.device_counter)
        || (publication.catalog_root != active.catalog_root
            && !publication
                .objects
                .iter()
                .any(|frame| frame.id().ok() == Some(publication.catalog_root)))
        || (active.audit_event_head.is_some() && publication.audit_event_head.is_none())
        || active
            .audit_event_head
            .is_some_and(|head| publication.audit_event_head == Some(head))
    {
        return Err(ApplicationError::IntegrityFailure);
    }
    let announcement = AnnouncementV1::decode(&publication.announcement)
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    if announcement.vault_id != active.vault_id
        || announcement.device_id != active.device_id
        || announcement.device_certificate != active.device_certificate_id
    {
        return Err(ApplicationError::IntegrityFailure);
    }
    Ok(())
}

fn decode_prepared(value: CborValue) -> Result<PreparedInitV1, ApplicationError> {
    let mut fields = closed_fields(value, &[1, 2, 3])?;
    PreparedInitV1::new(
        take_bytes(&mut fields, 1)?,
        decode_active(take_value(&mut fields, 2)?)?,
        decode_publication(take_value(&mut fields, 3)?)?,
    )
}

fn encode_active(value: &ActiveStateV1) -> Result<CborValue, ApplicationError> {
    Ok(CborValue::Map(vec![
        field(1, bytes(value.bootstrap_locator.as_bytes())),
        field(2, bytes(value.vault_id.as_bytes())),
        field(3, bytes(value.bootstrap_id.as_bytes())),
        field(4, bytes(value.authority_fingerprint.as_bytes())),
        field(5, bytes(value.device_id.as_bytes())),
        field(6, bytes(value.device_certificate_id.as_bytes())),
        field(7, bytes(&encode_frame(&value.device_certificate_frame)?)),
        field(8, encode_envelope(&value.local_secret)),
        field(9, encode_heads(&value.pinned_heads)),
        field(10, CborValue::Unsigned(value.last_device_counter)),
        field(11, bytes(value.catalog_root.as_bytes())),
        field(12, optional_bytes(value.audit_event_head)),
    ]))
}

fn decode_active(value: CborValue) -> Result<ActiveStateV1, ApplicationError> {
    let mut fields = closed_fields_legacy_or_current(
        value,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    )?;
    let audit_event_head = take_optional_fixed_if_present(&mut fields, 12)?.map(ObjectId::new);
    let active = ActiveStateV1::new(
        BootstrapLocator::new(take_fixed(&mut fields, 1)?),
        VaultId::new(take_fixed(&mut fields, 2)?),
        BootstrapId::new(take_fixed(&mut fields, 3)?),
        AuthorityFingerprint::new(take_fixed(&mut fields, 4)?),
        DeviceId::new(take_fixed(&mut fields, 5)?),
        ObjectId::new(take_fixed(&mut fields, 6)?),
        decode_frame(take_bytes(&mut fields, 7)?)?,
        decode_envelope(take_value(&mut fields, 8)?)?,
        decode_heads(take_value(&mut fields, 9)?)?,
        take_uint(&mut fields, 10)?,
        ObjectId::new(take_fixed(&mut fields, 11)?),
    )?;
    match audit_event_head {
        Some(head) => active.with_audit_event_head(head),
        None => Ok(active),
    }
}

fn encode_publication(value: &PublicationJournalV1) -> Result<CborValue, ApplicationError> {
    let objects = value
        .objects
        .iter()
        .map(encode_frame)
        .map(|result| result.map(CborValue::Bytes))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CborValue::Map(vec![
        field(1, CborValue::Array(objects)),
        field(2, bytes(&encode_frame(&value.commit)?)),
        field(3, bytes(&value.announcement)),
        field(4, encode_heads(&value.base_heads)),
        field(5, encode_heads(&value.expected_heads)),
        field(6, CborValue::Unsigned(value.device_counter)),
        field(7, bytes(value.catalog_root.as_bytes())),
        field(8, optional_bytes(value.audit_event_head)),
    ]))
}

fn decode_publication(value: CborValue) -> Result<PublicationJournalV1, ApplicationError> {
    let mut fields =
        closed_fields_legacy_or_current(value, &[1, 2, 3, 4, 5, 6, 7], &[1, 2, 3, 4, 5, 6, 7, 8])?;
    let audit_event_head = take_optional_fixed_if_present(&mut fields, 8)?.map(ObjectId::new);
    let object_values = take_array(&mut fields, 1)?;
    if object_values.is_empty() || object_values.len() > MAX_PUBLICATION_OBJECTS {
        return Err(ApplicationError::BoundExceeded);
    }
    let objects = object_values
        .into_iter()
        .map(bytes_value)
        .map(|result| result.and_then(decode_frame))
        .collect::<Result<Vec<_>, _>>()?;
    let commit = decode_frame(take_bytes(&mut fields, 2)?)?;
    let announcement = take_bytes(&mut fields, 3)?;
    let base_heads = decode_heads(take_value(&mut fields, 4)?)?;
    let expected_heads = decode_heads(take_value(&mut fields, 5)?)?;
    let device_counter = take_uint(&mut fields, 6)?;
    let catalog_root = ObjectId::new(take_fixed(&mut fields, 7)?);
    match audit_event_head {
        Some(head) => PublicationJournalV1::new_audit_only(
            objects,
            commit,
            announcement,
            base_heads,
            expected_heads,
            device_counter,
            catalog_root,
            head,
        ),
        None => PublicationJournalV1::new(
            objects,
            commit,
            announcement,
            base_heads,
            expected_heads,
            device_counter,
            catalog_root,
        ),
    }
}

fn encode_envelope(value: &AeadEnvelopeV1) -> CborValue {
    CborValue::Map(vec![
        field(1, CborValue::Unsigned(value.suite.into())),
        field(2, bytes(&value.nonce)),
        field(3, bytes(&value.ciphertext)),
        field(4, bytes(&value.tag)),
    ])
}

fn decode_envelope(value: CborValue) -> Result<AeadEnvelopeV1, ApplicationError> {
    let mut fields = closed_fields(value, &[1, 2, 3, 4])?;
    let suite = take_uint(&mut fields, 1)?;
    let suite = u16::try_from(suite).map_err(|_| ApplicationError::Unsupported)?;
    if suite != CRYPTO_SUITE_V1 {
        return Err(ApplicationError::Unsupported);
    }
    let value = AeadEnvelopeV1 {
        suite,
        nonce: take_fixed(&mut fields, 2)?,
        ciphertext: take_bytes(&mut fields, 3)?,
        tag: take_fixed(&mut fields, 4)?,
    };
    value
        .validate()
        .map_err(|_| ApplicationError::IntegrityFailure)?;
    Ok(value)
}

fn encode_heads(value: &PinnedHeads) -> CborValue {
    CborValue::Array(value.iter().map(|id| bytes(id.as_bytes())).collect())
}

fn decode_heads(value: CborValue) -> Result<PinnedHeads, ApplicationError> {
    let values = match value {
        CborValue::Array(values) => values,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    let ids = values
        .into_iter()
        .map(fixed_value)
        .map(|result| result.map(ObjectId::new))
        .collect::<Result<Vec<_>, _>>()?;
    if ids.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(ApplicationError::IntegrityFailure);
    }
    PinnedHeads::new(ids).map_err(|error| match error {
        coding_adventures_vault_pm_repository::RepositoryError::BoundExceeded => {
            ApplicationError::BoundExceeded
        }
        _ => ApplicationError::IntegrityFailure,
    })
}

fn encode_frame(value: &ObjectFrameV1) -> Result<Vec<u8>, ApplicationError> {
    value
        .encode()
        .map_err(|_| ApplicationError::IntegrityFailure)
}

fn decode_frame(value: Vec<u8>) -> Result<ObjectFrameV1, ApplicationError> {
    ObjectFrameV1::decode(&value).map_err(|_| ApplicationError::IntegrityFailure)
}

fn check_state_bound(encoded: &[u8]) -> Result<(), ApplicationError> {
    if encoded.len() > MAX_LOCAL_STATE_BYTES {
        Err(ApplicationError::BoundExceeded)
    } else {
        Ok(())
    }
}

fn closed_fields(
    value: CborValue,
    expected: &[u64],
) -> Result<BTreeMap<u64, CborValue>, ApplicationError> {
    let entries = match value {
        CborValue::Map(entries) => entries,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    if entries.len() != expected.len() {
        return Err(ApplicationError::IntegrityFailure);
    }
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let key = match key {
            CborValue::Unsigned(key) if expected.contains(&key) => key,
            _ => return Err(ApplicationError::IntegrityFailure),
        };
        if fields.insert(key, value).is_some() {
            return Err(ApplicationError::IntegrityFailure);
        }
    }
    Ok(fields)
}

fn closed_fields_legacy_or_current(
    value: CborValue,
    legacy: &[u64],
    current: &[u64],
) -> Result<BTreeMap<u64, CborValue>, ApplicationError> {
    let field_count = match &value {
        CborValue::Map(entries) => entries.len(),
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    if field_count == legacy.len() {
        closed_fields(value, legacy)
    } else {
        closed_fields(value, current)
    }
}

fn field(key: u64, value: CborValue) -> (CborValue, CborValue) {
    (CborValue::Unsigned(key), value)
}

fn bytes(value: &[u8]) -> CborValue {
    CborValue::Bytes(value.to_vec())
}

fn optional_bytes(value: Option<ObjectId>) -> CborValue {
    CborValue::Array(
        value
            .into_iter()
            .map(|value| bytes(value.as_bytes()))
            .collect(),
    )
}

fn take_value(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<CborValue, ApplicationError> {
    fields
        .remove(&key)
        .ok_or(ApplicationError::IntegrityFailure)
}

fn take_uint(fields: &mut BTreeMap<u64, CborValue>, key: u64) -> Result<u64, ApplicationError> {
    match take_value(fields, key)? {
        CborValue::Unsigned(value) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_optional_fixed_if_present<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Option<[u8; N]>, ApplicationError> {
    let Some(value) = fields.remove(&key) else {
        return Ok(None);
    };
    let values = match value {
        CborValue::Array(values) if values.len() <= 1 => values,
        _ => return Err(ApplicationError::IntegrityFailure),
    };
    values.into_iter().next().map(fixed_value).transpose()
}

fn take_bytes(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Vec<u8>, ApplicationError> {
    bytes_value(take_value(fields, key)?)
}

fn bytes_value(value: CborValue) -> Result<Vec<u8>, ApplicationError> {
    match value {
        CborValue::Bytes(value) => Ok(value),
        _ => Err(ApplicationError::IntegrityFailure),
    }
}

fn take_fixed<const N: usize>(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<[u8; N], ApplicationError> {
    fixed_value(take_value(fields, key)?)
}

fn fixed_value<const N: usize>(value: CborValue) -> Result<[u8; N], ApplicationError> {
    bytes_value(value)?
        .try_into()
        .map_err(|_| ApplicationError::IntegrityFailure)
}

fn take_array(
    fields: &mut BTreeMap<u64, CborValue>,
    key: u64,
) -> Result<Vec<CborValue>, ApplicationError> {
    match take_value(fields, key)? {
        CborValue::Array(values) => Ok(values),
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

impl From<coding_adventures_canonical_cbor::CborError> for ApplicationError {
    fn from(_: coding_adventures_canonical_cbor::CborError) -> Self {
        Self::IntegrityFailure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_canonical_cbor::encode;
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_format::{Argon2idParametersV1, Signature, CRYPTO_SUITE_V1};

    const VAULT_ID: VaultId = VaultId::new([0x11; 16]);
    const DEVICE_ID: DeviceId = DeviceId::new([0x22; 16]);

    fn frame(seed: u8) -> ObjectFrameV1 {
        ObjectFrameV1 {
            suite: CRYPTO_SUITE_V1,
            wrap_nonce: [seed; 24],
            wrapped_dek: [seed.wrapping_add(1); 32],
            wrap_tag: [seed.wrapping_add(2); 16],
            payload_nonce: [seed.wrapping_add(3); 24],
            ciphertext: vec![seed.wrapping_add(4); 3],
            payload_tag: [seed.wrapping_add(5); 16],
        }
    }

    fn envelope() -> AeadEnvelopeV1 {
        AeadEnvelopeV1 {
            suite: CRYPTO_SUITE_V1,
            nonce: [0x31; 24],
            ciphertext: vec![0x32; 12],
            tag: [0x33; 16],
        }
    }

    fn bootstrap() -> BootstrapV1 {
        let (authority_public, authority_secret) = generate_keypair(&[0x12; 32]);
        let value = BootstrapV1 {
            vault_id: VAULT_ID,
            generation: 0,
            previous_bootstrap: None,
            crypto_suite: CRYPTO_SUITE_V1,
            kdf: Argon2idParametersV1 {
                memory_kib: 8 * 1024,
                iterations: 1,
                lanes: 1,
                salt: [0x41; 16],
            },
            passphrase_root_wrap: AeadEnvelopeV1 {
                suite: CRYPTO_SUITE_V1,
                nonce: [0x42; 24],
                ciphertext: vec![0x43; 32],
                tag: [0x44; 16],
            },
            authority_public_key: PublicKey::new(authority_public),
            recovery_wraps: Vec::new(),
            signature: Signature::new([0x45; 64]),
        };
        let signature = sign(&value.signing_preimage().unwrap(), &authority_secret);
        value.with_signature(Signature::new(signature))
    }

    fn journal(
        objects: Vec<ObjectFrameV1>,
        commit: ObjectFrameV1,
        certificate_id: ObjectId,
        base_heads: PinnedHeads,
        counter: u64,
        catalog_root: ObjectId,
    ) -> PublicationJournalV1 {
        let commit_id = commit.id().unwrap();
        let announcement = AnnouncementV1 {
            vault_id: VAULT_ID,
            device_id: DEVICE_ID,
            device_counter: counter,
            commit_id,
            device_certificate: certificate_id,
            signature: Signature::new([0x51; 64]),
        }
        .encode()
        .unwrap();
        PublicationJournalV1::new(
            objects,
            commit,
            announcement,
            base_heads,
            PinnedHeads::new([commit_id]).unwrap(),
            counter,
            catalog_root,
        )
        .unwrap()
    }

    fn audit_only_journal(
        audit_event: ObjectFrameV1,
        commit: ObjectFrameV1,
        certificate_id: ObjectId,
        base_heads: PinnedHeads,
        counter: u64,
        catalog_root: ObjectId,
    ) -> PublicationJournalV1 {
        let audit_event_head = audit_event.id().unwrap();
        let commit_id = commit.id().unwrap();
        let announcement = AnnouncementV1 {
            vault_id: VAULT_ID,
            device_id: DEVICE_ID,
            device_counter: counter,
            commit_id,
            device_certificate: certificate_id,
            signature: Signature::new([0x52; 64]),
        }
        .encode()
        .unwrap();
        PublicationJournalV1::new_audit_only(
            vec![audit_event],
            commit,
            announcement,
            base_heads,
            PinnedHeads::new([commit_id]).unwrap(),
            counter,
            catalog_root,
            audit_event_head,
        )
        .unwrap()
    }

    fn prepared() -> PreparedInitV1 {
        let bootstrap = bootstrap().encode().unwrap();
        let decoded_bootstrap = BootstrapV1::decode(&bootstrap).unwrap();
        let bootstrap_id = decoded_bootstrap.id().unwrap();
        let certificate = frame(1);
        let certificate_id = certificate.id().unwrap();
        let catalog = frame(2);
        let catalog_root = catalog.id().unwrap();
        let publication = journal(
            vec![certificate.clone(), catalog],
            frame(3),
            certificate_id,
            PinnedHeads::empty(),
            1,
            catalog_root,
        );
        let active = ActiveStateV1::new(
            BootstrapLocator::new([0x10; 32]),
            VAULT_ID,
            bootstrap_id,
            AuthorityFingerprint::for_public_key(decoded_bootstrap.authority_public_key),
            DEVICE_ID,
            certificate_id,
            certificate,
            envelope(),
            publication.expected_heads().clone(),
            1,
            catalog_root,
        )
        .unwrap();
        PreparedInitV1::new(bootstrap, active, publication).unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn locators_and_authority_fingerprints_are_stable_and_redacted() {
        let locator = BootstrapLocator::new([7; 32]);
        assert_eq!(locator.as_bytes(), &[7; 32]);
        assert_eq!(format!("{locator:?}"), "BootstrapLocator(<redacted>)");
        let fingerprint = AuthorityFingerprint::for_public_key(PublicKey::new([8; 32]));
        assert_eq!(
            hex(fingerprint.as_bytes()),
            "c1ec85b68ef014471b3d712e7c35a474d6a65ecb85afc5fefee33a490e3e1275"
        );
        assert_eq!(
            format!("{fingerprint:?}"),
            "AuthorityFingerprint(<redacted>)"
        );
    }

    #[test]
    fn prepared_init_is_exact_round_trippable_and_reconstructs_publication() {
        let prepared = prepared();
        let state = LocalVaultStateV1::PreparedInit(prepared.clone());
        let encoded = state.encode().unwrap();
        assert_eq!(
            hex(&sha256(&encoded)),
            "d36bda7b6694cef9cefd7460cbb362b6d18b9b0e0c40a7fac147baac4348e011"
        );
        assert_eq!(LocalVaultStateV1::decode(&encoded).unwrap(), state);
        assert_eq!(format!("{state:?}"), "LocalVaultStateV1::PreparedInit");
        assert_eq!(
            format!("{:?}", prepared.publication().publication()),
            "Publication { object_count: 2, commit: \"<redacted>\", announcement: \"<redacted>\" }"
        );
    }

    #[test]
    fn active_and_pending_states_round_trip_without_changing_reserved_bytes() {
        let active = prepared().intended_active().clone();
        let active_state = LocalVaultStateV1::Active(active.clone());
        let active_bytes = active_state.encode().unwrap();
        assert_eq!(
            LocalVaultStateV1::decode(&active_bytes).unwrap(),
            active_state
        );

        let catalog = frame(8);
        let catalog_root = catalog.id().unwrap();
        let publication = journal(
            vec![catalog, frame(9)],
            frame(10),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            2,
            catalog_root,
        );
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        let encoded = pending.encode().unwrap();
        assert_eq!(LocalVaultStateV1::decode(&encoded).unwrap(), pending);
        assert_eq!(
            format!("{pending:?}"),
            "LocalVaultStateV1::PendingPublication"
        );
    }

    #[test]
    fn audit_head_is_journaled_backward_compatible_and_cannot_be_skipped() {
        let active = prepared().intended_active().clone();
        let CborValue::Map(mut legacy_active_fields) = encode_active(&active).unwrap() else {
            panic!("active state must be a map")
        };
        legacy_active_fields.retain(|(key, _)| key != &CborValue::Unsigned(12));
        assert_eq!(
            decode_active(CborValue::Map(legacy_active_fields)).unwrap(),
            active
        );
        let CborValue::Map(mut malformed_active_fields) = encode_active(&active).unwrap() else {
            panic!("active state must be a map")
        };
        malformed_active_fields.retain(|(key, _)| key != &CborValue::Unsigned(12));
        malformed_active_fields.push(field(
            12,
            CborValue::Array(vec![bytes(&[1; 32]), bytes(&[2; 32])]),
        ));
        assert_eq!(
            decode_active(CborValue::Map(malformed_active_fields)),
            Err(ApplicationError::IntegrityFailure)
        );

        let catalog = frame(8);
        let catalog_root = catalog.id().unwrap();
        let audit_event = frame(9);
        let audit_event_head = audit_event.id().unwrap();
        let publication = journal(
            vec![catalog, audit_event],
            frame(10),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            2,
            catalog_root,
        )
        .with_audit_event_head(audit_event_head)
        .unwrap();
        assert_eq!(publication.audit_event_head(), Some(audit_event_head));

        let CborValue::Map(mut legacy_publication_fields) =
            encode_publication(&publication).unwrap()
        else {
            panic!("publication must be a map")
        };
        legacy_publication_fields.retain(|(key, _)| key != &CborValue::Unsigned(8));
        assert_eq!(
            decode_publication(CborValue::Map(legacy_publication_fields))
                .unwrap()
                .audit_event_head(),
            None
        );

        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), publication.clone()).unwrap();
        let encoded = pending.encode().unwrap();
        assert_eq!(LocalVaultStateV1::decode(&encoded).unwrap(), pending);
        let audited = active.after_publication(&publication).unwrap();
        assert_eq!(audited.audit_event_head(), Some(audit_event_head));

        let next_catalog = frame(11);
        let next_catalog_root = next_catalog.id().unwrap();
        let unaudited = journal(
            vec![next_catalog],
            frame(12),
            audited.device_certificate_id(),
            audited.pinned_heads().clone(),
            3,
            next_catalog_root,
        );
        assert_eq!(
            LocalVaultStateV1::pending_publication(audited, unaudited),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn audit_only_journal_reuses_only_the_exact_active_catalog() {
        let active = prepared().intended_active().clone();
        let publication = audit_only_journal(
            frame(13),
            frame(14),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            2,
            active.catalog_root(),
        );
        assert_eq!(publication.objects().len(), 1);
        assert!(publication
            .objects()
            .iter()
            .all(|frame| frame.id().unwrap() != active.catalog_root()));
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), publication.clone()).unwrap();
        let encoded = pending.encode().unwrap();
        assert_eq!(LocalVaultStateV1::decode(&encoded).unwrap(), pending);
        let intended = active.after_publication(&publication).unwrap();
        assert_eq!(intended.catalog_root(), active.catalog_root());
        assert_eq!(intended.audit_event_head(), publication.audit_event_head());

        let wrong_catalog = audit_only_journal(
            frame(15),
            frame(16),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            2,
            ObjectId::new([0xee; 32]),
        );
        assert_eq!(
            LocalVaultStateV1::pending_publication(active, wrong_catalog),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn explicit_accessors_and_diagnostics_disclose_only_bounded_metadata() {
        let prepared = prepared();
        let active = prepared.intended_active();
        let publication = prepared.publication();
        let decoded_bootstrap = BootstrapV1::decode(prepared.bootstrap()).unwrap();

        assert_eq!(publication.objects().len(), 2);
        assert_eq!(
            publication.commit().id().unwrap(),
            publication.expected_heads().iter().next().copied().unwrap()
        );
        assert_eq!(
            AnnouncementV1::decode(publication.announcement())
                .unwrap()
                .device_counter,
            publication.device_counter()
        );
        assert!(publication.base_heads().is_empty());
        assert_eq!(publication.catalog_root(), active.catalog_root());
        assert_eq!(publication.audit_event_head(), None);
        assert!(format!("{publication:?}").starts_with("PublicationJournalV1"));

        assert_eq!(
            active.bootstrap_locator(),
            BootstrapLocator::new([0x10; 32])
        );
        assert_eq!(active.vault_id(), VAULT_ID);
        assert_eq!(active.bootstrap_id(), decoded_bootstrap.id().unwrap());
        assert_eq!(
            active.authority_fingerprint(),
            AuthorityFingerprint::for_public_key(decoded_bootstrap.authority_public_key)
        );
        assert_eq!(active.device_id(), DEVICE_ID);
        assert_eq!(
            active.device_certificate_frame().id().unwrap(),
            active.device_certificate_id()
        );
        assert_eq!(active.local_secret(), &envelope());
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert_eq!(active.last_device_counter(), 1);
        assert_eq!(active.audit_event_head(), None);
        assert_eq!(
            format!("{active:?}"),
            "ActiveStateV1 { head_count: 1, last_device_counter: 1, audit_enabled: false, .. }"
        );
        assert!(format!("{prepared:?}").starts_with("PreparedInitV1"));
        assert_eq!(
            format!("{:?}", LocalVaultStateV1::Active(active.clone())),
            "LocalVaultStateV1::Active"
        );

        assert_eq!(
            BootstrapStoreError::Unavailable.to_string(),
            "bootstrap store unavailable"
        );
        assert_eq!(
            BootstrapStoreError::Corruption.to_string(),
            "bootstrap store corruption"
        );
        assert_eq!(
            LocalStateStoreError::Unavailable.to_string(),
            "local state store unavailable"
        );
        assert_eq!(
            LocalStateStoreError::Corruption.to_string(),
            "local state store corruption"
        );
    }

    #[test]
    fn publication_and_active_constructors_reject_inconsistent_relations() {
        let initial = prepared();
        let active = initial.intended_active().clone();
        let publication = initial.publication();
        assert_eq!(
            PublicationJournalV1::new(
                Vec::new(),
                publication.commit.clone(),
                publication.announcement.clone(),
                PinnedHeads::empty(),
                publication.expected_heads.clone(),
                1,
                publication.catalog_root,
            ),
            Err(ApplicationError::BoundExceeded)
        );
        assert_eq!(
            PublicationJournalV1::new(
                publication.objects.clone(),
                publication.commit.clone(),
                publication.announcement.clone(),
                PinnedHeads::empty(),
                publication.expected_heads.clone(),
                0,
                publication.catalog_root,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            PublicationJournalV1::new(
                publication.objects.clone(),
                publication.commit.clone(),
                publication.announcement.clone(),
                PinnedHeads::empty(),
                PinnedHeads::new([ObjectId::new([0xee; 32])]).unwrap(),
                1,
                publication.catalog_root,
            ),
            Err(ApplicationError::IntegrityFailure)
        );
        let duplicate = publication.objects[0].clone();
        assert_eq!(
            PublicationJournalV1::new(
                vec![duplicate.clone(), duplicate],
                publication.commit.clone(),
                publication.announcement.clone(),
                PinnedHeads::empty(),
                publication.expected_heads.clone(),
                1,
                publication.objects[0].id().unwrap(),
            ),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            publication
                .clone()
                .with_audit_event_head(ObjectId::new([0xfa; 32])),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            publication
                .clone()
                .with_audit_event_head(publication.catalog_root()),
            Err(ApplicationError::IntegrityFailure)
        );

        assert_eq!(
            ActiveStateV1::new(
                active.bootstrap_locator,
                active.vault_id,
                active.bootstrap_id,
                active.authority_fingerprint,
                active.device_id,
                ObjectId::new([0xff; 32]),
                active.device_certificate_frame.clone(),
                active.local_secret.clone(),
                active.pinned_heads.clone(),
                active.last_device_counter,
                active.catalog_root,
            ),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            active.clone().with_audit_event_head(active.catalog_root()),
            Err(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            active
                .clone()
                .with_audit_event_head(active.device_certificate_id()),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn state_relations_fail_closed() {
        let initial = prepared();
        let active = initial.intended_active().clone();
        let catalog = frame(8);
        let catalog_root = catalog.id().unwrap();
        let wrong_base = journal(
            vec![catalog],
            frame(10),
            active.device_certificate_id(),
            PinnedHeads::empty(),
            2,
            catalog_root,
        );
        assert_eq!(
            LocalVaultStateV1::pending_publication(active.clone(), wrong_base),
            Err(ApplicationError::IntegrityFailure)
        );

        let stale_counter = journal(
            vec![frame(11)],
            frame(12),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            1,
            frame(11).id().unwrap(),
        );
        assert_eq!(
            LocalVaultStateV1::pending_publication(active, stale_counter),
            Err(ApplicationError::IntegrityFailure)
        );

        let identity_fixture = prepared();
        let active = identity_fixture.intended_active().clone();
        let catalog = frame(13);
        let mut wrong_identity = journal(
            vec![catalog.clone()],
            frame(14),
            active.device_certificate_id(),
            active.pinned_heads().clone(),
            2,
            catalog.id().unwrap(),
        );
        let announcement = AnnouncementV1::decode(&wrong_identity.announcement).unwrap();
        wrong_identity.announcement = AnnouncementV1 {
            vault_id: VaultId::new([0xfe; 16]),
            ..announcement
        }
        .encode()
        .unwrap();
        assert_eq!(
            LocalVaultStateV1::pending_publication(active, wrong_identity),
            Err(ApplicationError::IntegrityFailure)
        );

        let prepared = prepared();
        let mut invalid_bootstrap = BootstrapV1::decode(prepared.bootstrap()).unwrap();
        invalid_bootstrap.signature = Signature::new([0; 64]);
        let invalid_bootstrap = invalid_bootstrap.encode().unwrap();
        let mut intended = prepared.intended_active().clone();
        intended.bootstrap_id = BootstrapV1::decode(&invalid_bootstrap)
            .unwrap()
            .id()
            .unwrap();
        assert_eq!(
            PreparedInitV1::new(invalid_bootstrap, intended, prepared.publication().clone()),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn strict_decode_rejects_unknown_states_noncanonical_data_and_bounds() {
        assert_eq!(
            LocalVaultStateV1::decode(&[0xff]),
            Err(ApplicationError::IntegrityFailure)
        );
        let unknown = encode(&CborValue::Map(vec![
            field(1, CborValue::Unsigned(VERSION)),
            field(2, CborValue::Unsigned(99)),
            field(3, CborValue::Map(Vec::new())),
        ]));
        assert_eq!(
            LocalVaultStateV1::decode(&unknown),
            Err(ApplicationError::Unsupported)
        );
        assert_eq!(
            LocalVaultStateV1::decode(&vec![0; MAX_LOCAL_STATE_BYTES + 1]),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn store_errors_are_closed_and_payload_free() {
        assert_eq!(
            BootstrapStoreError::Conflict.to_string(),
            "bootstrap store conflict"
        );
        assert_eq!(
            LocalStateStoreError::ConcurrentHost.to_string(),
            "local state compare-exchange conflict"
        );
        assert_eq!(
            format!("{:?}", BootstrapStoreError::Corruption),
            "Corruption"
        );
    }
}
