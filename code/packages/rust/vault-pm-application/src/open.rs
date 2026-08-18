use crate::initialize::{unlock_active_material, UnlockedActiveMaterial};
use crate::mutation::{
    activate_audit_epoch, add_item, delete_item, import_opened_portable_snapshot,
    merge_item_conflict, publish_audited_access, replace_item, resolve_item_conflict, restore_item,
    AddItemRandomnessV1, AuditedAccessRandomnessV1, DeleteItemRandomnessV1,
    PortableImportRandomnessV1, ReplaceItemRandomnessV1, ResolveItemConflictRandomnessV1,
    RestoreItemRandomnessV1,
};
use crate::search::SearchProjectionV1;
use crate::{
    open_object, ActiveStateV1, ApplicationError, ApplicationRepository,
    ApplicationRepositoryError, ApplicationRepositoryFactory, BootstrapLocator, BootstrapStore,
    BootstrapStoreError, CatalogV1, LocalSecretV1, LocalStateStore, LocalStateStoreError,
    LocalVaultStateV1, ObjectKind, RevealedSecretV1, SecretDisclosureIntentV1, SecretFieldV1,
    V1Keys,
};
use coding_adventures_vault_pm_audit::{AuditActionV1, AuditOutcomeV1};
use coding_adventures_vault_pm_domain::{
    CollectionId, ItemCandidate, ItemDocument, ItemId, ItemState, OperationId, RedactedItemView,
    RevisionId,
};
use coding_adventures_vault_pm_format::{DeviceId, ObjectId, VaultId};
use coding_adventures_vault_pm_repository::{OpenReport, PinnedHeads};
use coding_adventures_vault_records::{
    decode_record, encode_opaque, AnyRecord, ApiKey, Card, DatabaseCredential, Login, SecureNote,
    TotpSeed,
};
use coding_adventures_zeroize::{Zeroize, Zeroizing};
use core::fmt::{self, Debug, Formatter};
use std::collections::{BTreeMap, BTreeSet};

use crate::codec::{MAX_CANDIDATES_PER_ITEM, MAX_CATALOG_ENTRIES};

/// Default maximum number of historical revisions returned for one item.
pub const DEFAULT_ITEM_HISTORY_LIMIT: usize = 100;
/// Hard maximum number of historical revisions returned for one item.
pub const MAX_ITEM_HISTORY_LIMIT: usize = 4_096;
const MAX_LOGIN_URLS: usize = 16;
/// Largest complete API-key scope line accepted behind the audited boundary.
const MAX_API_KEY_SCOPE_LINE_BYTES: usize = 2_048;
/// Largest number of comma-separated components in one API-key scope line.
const MAX_API_KEY_SCOPES: usize = 64;
/// Largest single API-key scope component in UTF-8 bytes.
const MAX_API_KEY_SCOPE_BYTES: usize = 256;
/// Largest database engine identifier accepted behind the audited boundary.
const MAX_DATABASE_ENGINE_BYTES: usize = 32;
/// Largest canonical unpadded Base32 TOTP seed line, in ASCII characters.
///
/// 256 Base32 characters carry exactly 160 bytes, the ceiling VLT-PM29 places
/// on a stored seed, so this one bound enforces both the line and byte limits.
const MAX_TOTP_SECRET_BASE32_CHARS: usize = 256;
/// Largest TOTP time step accepted behind the audited boundary, in seconds.
const MAX_TOTP_PERIOD_SECONDS: u32 = 3_600;
/// Largest authored opaque-record payload line, in lowercase hex characters.
///
/// This matches the host's fixed 1,024-byte hidden-secret line ceiling. Two
/// characters spell one byte, so the same bound also caps a stored authored
/// payload at 512 bytes, and no separate byte limit is needed.
const MAX_OPAQUE_PAYLOAD_HEX_CHARS: usize = 1_024;

/// Owned wipe-on-drop caller fields for one complete login edit.
pub struct LoginEditInputV1 {
    title: Zeroizing<String>,
    username: Zeroizing<String>,
    password: Zeroizing<String>,
    urls: Vec<Zeroizing<String>>,
    notes: Option<Zeroizing<String>>,
}

/// Owned wipe-on-drop caller fields for one complete secure-note merge.
pub struct SecureNoteConflictMergeInputV1 {
    title: Zeroizing<String>,
    body: Zeroizing<String>,
}

/// Owned wipe-on-drop caller fields for one complete payment-card merge.
pub struct CardConflictMergeInputV1 {
    title: Zeroizing<String>,
    holder: Zeroizing<String>,
    number: Zeroizing<String>,
    expiry_month: Zeroizing<String>,
    expiry_year: Zeroizing<String>,
    cvv: Zeroizing<String>,
    billing_zip: Option<Zeroizing<String>>,
}

impl CardConflictMergeInputV1 {
    /// Take the complete bounded payment-card form collected by a trusted host.
    pub const fn new(
        title: Zeroizing<String>,
        holder: Zeroizing<String>,
        number: Zeroizing<String>,
        expiry_month: Zeroizing<String>,
        expiry_year: Zeroizing<String>,
        cvv: Zeroizing<String>,
        billing_zip: Option<Zeroizing<String>>,
    ) -> Self {
        Self {
            title,
            holder,
            number,
            expiry_month,
            expiry_year,
            cvv,
            billing_zip,
        }
    }
}

/// Owned wipe-on-drop caller fields for one complete API-key merge.
///
/// The scope line and expiry arrive exactly as the terminal collected them so
/// that their closed shape rules are re-checked behind the audited boundary
/// rather than being trusted from the host.
pub struct ApiKeyConflictMergeInputV1 {
    label: Zeroizing<String>,
    service: Zeroizing<String>,
    token: Zeroizing<String>,
    scopes: Zeroizing<String>,
    expiry: Zeroizing<String>,
}

impl ApiKeyConflictMergeInputV1 {
    /// Take the complete bounded API-key form collected by a trusted host.
    pub const fn new(
        label: Zeroizing<String>,
        service: Zeroizing<String>,
        token: Zeroizing<String>,
        scopes: Zeroizing<String>,
        expiry: Zeroizing<String>,
    ) -> Self {
        Self {
            label,
            service,
            token,
            scopes,
            expiry,
        }
    }
}

/// Owned wipe-on-drop caller fields for one complete database-credential merge.
///
/// The engine and port arrive exactly as the terminal collected them so that
/// their closed shape rules are re-checked behind the audited boundary rather
/// than being trusted from the host.
pub struct DatabaseCredentialConflictMergeInputV1 {
    label: Zeroizing<String>,
    engine: Zeroizing<String>,
    host: Zeroizing<String>,
    port: Zeroizing<String>,
    database: Option<Zeroizing<String>>,
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl DatabaseCredentialConflictMergeInputV1 {
    /// Take the complete bounded database-credential form from a trusted host.
    pub const fn new(
        label: Zeroizing<String>,
        engine: Zeroizing<String>,
        host: Zeroizing<String>,
        port: Zeroizing<String>,
        database: Option<Zeroizing<String>>,
        username: Zeroizing<String>,
        password: Zeroizing<String>,
    ) -> Self {
        Self {
            label,
            engine,
            host,
            port,
            database,
            username,
            password,
        }
    }
}

/// Owned wipe-on-drop caller fields for one complete TOTP merge.
///
/// Every field arrives exactly as the terminal collected it — the seed as its
/// Base32 line rather than as decoded bytes — so that the closed shape rules of
/// `VLT-PM29-cli-totp-create.md` are re-checked behind the audited boundary
/// rather than being trusted from the host.
pub struct TotpConflictMergeInputV1 {
    label: Zeroizing<String>,
    issuer: Option<Zeroizing<String>>,
    secret: Zeroizing<String>,
    algorithm: Zeroizing<String>,
    digits: Zeroizing<String>,
    period: Zeroizing<String>,
}

impl TotpConflictMergeInputV1 {
    /// Take the complete bounded TOTP form collected by a trusted host.
    pub const fn new(
        label: Zeroizing<String>,
        issuer: Option<Zeroizing<String>>,
        secret: Zeroizing<String>,
        algorithm: Zeroizing<String>,
        digits: Zeroizing<String>,
        period: Zeroizing<String>,
    ) -> Self {
        Self {
            label,
            issuer,
            secret,
            algorithm,
            digits,
            period,
        }
    }
}

/// Owned wipe-on-drop caller field for one complete opaque-record merge.
///
/// An opaque record is the one record type this product has no schema for, so
/// there is no field list to collect: the whole canonical-CBOR payload is the
/// authored value. It arrives exactly as the terminal collected it — as a
/// lowercase hexadecimal line rather than as decoded bytes — so that the closed
/// shape rules of `VLT-PM39-cli-authored-opaque-record-conflict-merge.md` are
/// checked behind the audited boundary rather than being trusted from the host.
///
/// The record's content type is deliberately absent. An item's schema is
/// immutable across its whole history, so the merged record can only carry the
/// base candidate's content type, and offering it as an authored field would
/// only offer a value that must be rejected.
pub struct OpaqueConflictMergeInputV1 {
    payload: Zeroizing<String>,
}

impl OpaqueConflictMergeInputV1 {
    /// Take the complete bounded opaque payload line collected by a trusted
    /// host.
    pub const fn new(payload: Zeroizing<String>) -> Self {
        Self { payload }
    }
}

impl SecureNoteConflictMergeInputV1 {
    /// Take the complete bounded secure-note form collected by a trusted host.
    pub const fn new(title: Zeroizing<String>, body: Zeroizing<String>) -> Self {
        Self { title, body }
    }
}

impl LoginEditInputV1 {
    /// Take the complete bounded login form collected by a trusted host.
    pub const fn new(
        title: Zeroizing<String>,
        username: Zeroizing<String>,
        password: Zeroizing<String>,
        urls: Vec<Zeroizing<String>>,
        notes: Option<Zeroizing<String>>,
    ) -> Self {
        Self {
            title,
            username,
            password,
            urls,
            notes,
        }
    }
}

struct LoginEditFailureAuditV1 {
    wall_time_ms: u64,
    randomness: AuditedAccessRandomnessV1,
}

type LoginEditSelectionV1 = (RevisionId, Zeroizing<ItemDocument>);
type LoginEditPreconditionV1 = (
    Result<LoginEditSelectionV1, ApplicationError>,
    Option<RevisionId>,
);

/// Opaque application-owned state for one validated login edit ceremony.
///
/// The current revision capability and secret-bearing document never cross
/// this boundary. The value intentionally implements neither `Debug` nor
/// `Clone` and is consumed by completion or audited failure recording.
pub struct LoginEditPreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    expected_revision: RevisionId,
    current: Zeroizing<ItemDocument>,
    failure_audit: Option<LoginEditFailureAuditV1>,
}

/// Audited result of validating one login edit target.
pub enum AuditedLoginEditPreparationV1 {
    /// The application retains the current revision and document for editing.
    Ready(Box<LoginEditPreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedLoginEditPreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<LoginEditPreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

impl LoginEditPreparationV1 {
    fn replacement_document(
        &self,
        input: LoginEditInputV1,
        updated_at_ms: u64,
    ) -> Result<ItemDocument, ApplicationError> {
        replacement_login_document(&self.current, input, updated_at_ms)
    }

    /// Complete a validated pre-audit login edit as one replacement mutation.
    pub fn complete(
        self,
        input: LoginEditInputV1,
        wall_time_ms: u64,
        randomness: ReplaceItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let document = match self.replacement_document(input, wall_time_ms) {
            Ok(document) => document,
            Err(error) if self.failure_audit.is_some() => {
                let failed = self.publish_audited_failure(error, local_state_store)?;
                return match failed.into_operation() {
                    Ok(()) => Err(ApplicationError::InternalInvariant),
                    Err(error) => Err(error),
                };
            }
            Err(error) => return Err(error),
        };
        self.session.replace_item(
            self.expected_revision,
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Complete an audited login edit, publishing failure before any error.
    pub fn complete_audited(
        self,
        input: LoginEditInputV1,
        randomness: ReplaceItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let wall_time_ms = self
            .failure_audit
            .as_ref()
            .ok_or(ApplicationError::InvalidInput)?
            .wall_time_ms;
        let document = match self.replacement_document(input, wall_time_ms) {
            Ok(document) => document,
            Err(error) => return self.publish_audited_failure(error, local_state_store),
        };
        let active = self.session.replace_item(
            self.expected_revision,
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let audit = self.failure_audit.ok_or(ApplicationError::InvalidInput)?;
        self.session.finish_audited_access(
            AuditActionV1::ItemUpdate,
            Some(self.item_id),
            Some(self.expected_revision),
            audit.wall_time_ms,
            audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

fn replacement_login_document(
    current: &ItemDocument,
    input: LoginEditInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::Login(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    if input.urls.len() > MAX_LOGIN_URLS {
        return Err(ApplicationError::InvalidInput);
    }
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        AnyRecord::Login(Login {
            title: input.title.into_inner(),
            username: input.username.into_inner(),
            password: input.password.into_inner(),
            urls: input.urls.into_iter().map(Zeroizing::into_inner).collect(),
            notes: input.notes.map(Zeroizing::into_inner),
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

struct ConflictMergeFailureAuditV1 {
    wall_time_ms: u64,
    randomness: AuditedAccessRandomnessV1,
}

/// Opaque application-owned state for one validated authored login conflict
/// merge.
///
/// The base revision and complete secret-bearing base document never cross
/// this boundary. The value intentionally implements neither `Debug` nor
/// `Clone` and is consumed by completion or audited failure recording.
pub struct LoginConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Opaque application-owned state for one validated authored secure-note
/// conflict merge.
pub struct SecureNoteConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Opaque application-owned state for one validated authored payment-card
/// conflict merge.
pub struct CardConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Audited result of validating one authored payment-card conflict merge.
pub enum AuditedCardConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<CardConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedCardConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<CardConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Opaque application-owned state for one validated authored API-key conflict
/// merge.
pub struct ApiKeyConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Audited result of validating one authored API-key conflict merge.
pub enum AuditedApiKeyConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<ApiKeyConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedApiKeyConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<ApiKeyConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Opaque application-owned state for one validated authored
/// database-credential conflict merge.
pub struct DatabaseCredentialConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Audited result of validating one authored database-credential merge.
pub enum AuditedDatabaseCredentialConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<DatabaseCredentialConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedDatabaseCredentialConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(
        self,
    ) -> Result<DatabaseCredentialConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Opaque application-owned state for one validated authored TOTP conflict
/// merge.
pub struct TotpConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Audited result of validating one authored TOTP conflict merge target.
pub enum AuditedTotpConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<TotpConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedTotpConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<TotpConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Opaque application-owned state for one validated authored opaque-record
/// conflict merge.
///
/// "Opaque" is doing two unrelated jobs in that sentence, and both are
/// deliberate: the preparation is opaque to the CLI in the same way every other
/// merge preparation is, and the record it will replace is opaque to this
/// product because its content type is not one of the six first-party schemas.
pub struct OpaqueConflictMergePreparationV1 {
    session: UnlockedVaultV1,
    item_id: ItemId,
    base: Zeroizing<ItemDocument>,
    failure_audit: ConflictMergeFailureAuditV1,
}

/// Audited result of validating one authored opaque-record conflict merge
/// target.
pub enum AuditedOpaqueConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<OpaqueConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedOpaqueConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<OpaqueConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Audited result of validating one authored secure-note conflict merge target.
pub enum AuditedSecureNoteConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<SecureNoteConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedSecureNoteConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(
        self,
    ) -> Result<SecureNoteConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

/// Audited result of validating one authored login conflict merge target.
pub enum AuditedLoginConflictMergePreparationV1 {
    /// The application retains the base document and complete conflict set.
    Ready(Box<LoginConflictMergePreparationV1>),
    /// The closed precondition failure and its next owner state are durable.
    Failed(Box<crate::AuditedAccessResultV1<()>>),
}

impl AuditedLoginConflictMergePreparationV1 {
    /// Return the opaque preparation or its already-durable operation failure.
    pub fn into_preparation(self) -> Result<LoginConflictMergePreparationV1, ApplicationError> {
        match self {
            Self::Ready(preparation) => Ok(*preparation),
            Self::Failed(failure) => match failure.into_operation() {
                Ok(()) => Err(ApplicationError::InternalInvariant),
                Err(error) => Err(error),
            },
        }
    }
}

impl LoginConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the user-authored login merge, publishing validation failure
    /// before returning it and success atomically with the merged revision.
    pub fn complete_audited(
        self,
        input: LoginEditInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document =
            match replacement_login_document(&self.base, input, self.failure_audit.wall_time_ms) {
                Ok(document) => document,
                Err(error) => return self.publish_audited_failure(error, local_state_store),
            };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

impl SecureNoteConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the user-authored secure-note merge and its atomic audit event.
    pub fn complete_audited(
        self,
        input: SecureNoteConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document = match replacement_secure_note_document(
            &self.base,
            input,
            self.failure_audit.wall_time_ms,
        ) {
            Ok(document) => document,
            Err(error) => return self.publish_audited_failure(error, local_state_store),
        };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

impl CardConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the user-authored payment-card merge and its atomic audit event.
    pub fn complete_audited(
        self,
        input: CardConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document =
            match replacement_card_document(&self.base, input, self.failure_audit.wall_time_ms) {
                Ok(document) => document,
                Err(error) => return self.publish_audited_failure(error, local_state_store),
            };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

fn replacement_card_document(
    current: &ItemDocument,
    input: CardConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::Card(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    if !(8..=19).contains(&input.number.len())
        || !input.number.bytes().all(|byte| byte.is_ascii_digit())
        || !(3..=4).contains(&input.cvv.len())
        || !input.cvv.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ApplicationError::InvalidInput);
    }
    let expiry_month = input
        .expiry_month
        .parse::<u8>()
        .map_err(|_| ApplicationError::InvalidInput)?;
    if !(1..=12).contains(&expiry_month) || expiry_month.to_string() != input.expiry_month.as_str()
    {
        return Err(ApplicationError::InvalidInput);
    }
    if input.expiry_year.len() != 4 || !input.expiry_year.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ApplicationError::InvalidInput);
    }
    let expiry_year = input
        .expiry_year
        .parse::<u16>()
        .map_err(|_| ApplicationError::InvalidInput)?;
    if expiry_year == 0 {
        return Err(ApplicationError::InvalidInput);
    }
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        AnyRecord::Card(Card {
            title: input.title.into_inner(),
            holder: input.holder.into_inner(),
            number: input.number.into_inner(),
            expiry_month,
            expiry_year,
            cvv: input.cvv.into_inner(),
            billing_zip: input.billing_zip.map(Zeroizing::into_inner),
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

impl ApiKeyConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the user-authored API-key merge and its atomic audit event.
    pub fn complete_audited(
        self,
        input: ApiKeyConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document = match replacement_api_key_document(
            &self.base,
            input,
            self.failure_audit.wall_time_ms,
        ) {
            Ok(document) => document,
            Err(error) => return self.publish_audited_failure(error, local_state_store),
        };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

/// Split one already-bounded API-key scope line into its closed component list.
///
/// The line is authoritative exactly as typed: it is split only on commas, and
/// every component must already be trimmed. Rejecting untrimmed components
/// instead of silently trimming them keeps one scope line from denoting two
/// different records depending on who normalized it.
fn parse_api_key_scope_line(line: &str) -> Result<Vec<String>, ApplicationError> {
    if line.len() > MAX_API_KEY_SCOPE_LINE_BYTES {
        return Err(ApplicationError::InvalidInput);
    }
    if line.is_empty() {
        return Ok(Vec::new());
    }
    let mut seen = BTreeSet::new();
    let mut scopes: Vec<String> = Vec::new();
    for scope in line.split(',') {
        if scopes.len() == MAX_API_KEY_SCOPES
            || scope.is_empty()
            || scope.trim() != scope
            || scope.len() > MAX_API_KEY_SCOPE_BYTES
            || !seen.insert(scope)
        {
            // A rejected line is still authored user data. The components
            // accepted before the rejection are owned plaintext copies, and on
            // this path they never reach the zeroizing record that would wipe
            // them, so wipe them here rather than freeing them intact.
            scopes.iter_mut().for_each(Zeroize::zeroize);
            return Err(ApplicationError::InvalidInput);
        }
        scopes.push(scope.to_owned());
    }
    Ok(scopes)
}

/// Interpret one already-bounded API-key expiry line as optional Unix seconds.
///
/// An empty line means "no expiry". Anything else must be one canonical
/// unsigned decimal: no sign, no leading zero, and not zero itself, so a single
/// stored instant always has exactly one accepted spelling.
fn parse_api_key_expiry_line(line: &str) -> Result<Option<u64>, ApplicationError> {
    if line.is_empty() {
        return Ok(None);
    }
    if line.starts_with('0') || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApplicationError::InvalidInput);
    }
    let seconds = line
        .parse::<u64>()
        .map_err(|_| ApplicationError::InvalidInput)?;
    (seconds != 0)
        .then_some(Some(seconds))
        .ok_or(ApplicationError::InvalidInput)
}

fn replacement_api_key_document(
    current: &ItemDocument,
    input: ApiKeyConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::ApiKey(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    let expires_at = parse_api_key_expiry_line(&input.expiry)?;
    // Parsed last so that on the success path the owned plaintext scope copies
    // are handed straight to the zeroizing record with no fallible step in
    // between; the parser wipes them itself when it rejects the line.
    let scopes = parse_api_key_scope_line(&input.scopes)?;
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        AnyRecord::ApiKey(ApiKey {
            label: input.label.into_inner(),
            service: input.service.into_inner(),
            token: input.token.into_inner(),
            scopes,
            expires_at,
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

impl DatabaseCredentialConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the authored database-credential merge and its atomic event.
    pub fn complete_audited(
        self,
        input: DatabaseCredentialConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document = match replacement_database_credential_document(
            &self.base,
            input,
            self.failure_audit.wall_time_ms,
        ) {
            Ok(document) => document,
            Err(error) => return self.publish_audited_failure(error, local_state_store),
        };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

/// Check one already-bounded database engine against its closed identifier
/// shape.
///
/// The identifier is provider neutral by construction: a lowercase ASCII letter
/// followed only by lowercase letters, digits, `-`, or `_`. Accepting exactly
/// one spelling per engine keeps `postgres`, `Postgres`, and `postgres ` from
/// denoting three different engines in the same vault, and the closed alphabet
/// leaves no room for a value that a later consumer might read as a URL scheme,
/// a path, or a driver argument.
fn validate_database_engine(engine: &str) -> Result<(), ApplicationError> {
    let mut bytes = engine.bytes();
    let Some(first) = bytes.next() else {
        return Err(ApplicationError::InvalidInput);
    };
    if engine.len() > MAX_DATABASE_ENGINE_BYTES
        || !first.is_ascii_lowercase()
        || !bytes
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
    {
        return Err(ApplicationError::InvalidInput);
    }
    Ok(())
}

/// Interpret one already-bounded database port line as a TCP port.
///
/// The line must be one canonical unsigned decimal in `1..=65535`: no sign, no
/// leading zero, and not zero itself, so a single stored port always has
/// exactly one accepted spelling.
fn parse_database_port_line(line: &str) -> Result<u16, ApplicationError> {
    if line.starts_with('0') || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApplicationError::InvalidInput);
    }
    let port = line
        .parse::<u16>()
        .map_err(|_| ApplicationError::InvalidInput)?;
    (port != 0)
        .then_some(port)
        .ok_or(ApplicationError::InvalidInput)
}

fn replacement_database_credential_document(
    current: &ItemDocument,
    input: DatabaseCredentialConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::DatabaseCredential(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    validate_database_engine(&input.engine)?;
    let port = parse_database_port_line(&input.port)?;
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        AnyRecord::DatabaseCredential(DatabaseCredential {
            label: input.label.into_inner(),
            engine: input.engine.into_inner(),
            host: input.host.into_inner(),
            port,
            database: input.database.map(Zeroizing::into_inner),
            username: input.username.into_inner(),
            password: input.password.into_inner(),
            // An authored credential is static. VLT-PM28 already rules that a
            // locally typed credential has no lease, and carrying a base
            // candidate's lease forward would attach dynamic-issuance state to
            // a secret that no issuer ever vouched for.
            lease_id: None,
            expires_at: None,
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

impl TotpConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the authored TOTP merge and its atomic event.
    pub fn complete_audited(
        self,
        input: TotpConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document =
            match replacement_totp_document(&self.base, input, self.failure_audit.wall_time_ms) {
                Ok(document) => document,
                Err(error) => return self.publish_audited_failure(error, local_state_store),
            };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

/// Check one already-bounded TOTP algorithm line against its closed set.
///
/// The three names are the only HMAC constructions RFC 6238 defines for TOTP,
/// and they are accepted in exactly one spelling each so that `SHA1` and `sha1`
/// cannot denote two different records in the same vault.
fn validate_totp_algorithm(line: &str) -> Result<(), ApplicationError> {
    matches!(line, "SHA1" | "SHA256" | "SHA512")
        .then_some(())
        .ok_or(ApplicationError::InvalidInput)
}

/// Interpret one already-bounded TOTP digit-count line.
///
/// Six and eight are the only widths interoperable authenticators emit. The
/// record schema itself admits `4..=10`, so this narrower closed set is the
/// authored form's own rule, matching `VLT-PM29-cli-totp-create.md`.
fn parse_totp_digits_line(line: &str) -> Result<u8, ApplicationError> {
    match line {
        "6" => Ok(6),
        "8" => Ok(8),
        _ => Err(ApplicationError::InvalidInput),
    }
}

/// Interpret one already-bounded TOTP period line as a time step in seconds.
///
/// The line must be one canonical unsigned decimal in `1..=3600`: no sign, no
/// leading zero, and not zero itself, so a single stored period always has
/// exactly one accepted spelling.
fn parse_totp_period_line(line: &str) -> Result<u32, ApplicationError> {
    if line.starts_with('0') || !line.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApplicationError::InvalidInput);
    }
    let period = line
        .parse::<u32>()
        .map_err(|_| ApplicationError::InvalidInput)?;
    (period != 0 && period <= MAX_TOTP_PERIOD_SECONDS)
        .then_some(period)
        .ok_or(ApplicationError::InvalidInput)
}

/// Decode one already-bounded TOTP seed line from canonical unpadded Base32.
///
/// Base32 packs five bits per character, so eight characters carry five bytes:
///
/// ```text
///   characters  J  B  S  W  Y  3  D  P
///   5-bit       01001 00001 11001 10110 11000 11011 00011 01111
///   8-bit       01001000 01110011 01101100 01101000 11011001 111
///   bytes       'H'      's'      'l'      'h'      0xd9     ^^^ 3 unused bits
/// ```
///
/// The three bits left over at the end must be zero: a nonzero remainder means
/// the line could not have been produced by encoding any byte string, so it is
/// not a spelling of a seed at all. Re-encoding the decoded bytes and requiring
/// the exact typed line back enforces the same one-spelling-per-seed rule the
/// rest of this module applies to ports, expiries, and engines — it rejects
/// lowercase, `=` padding, and impossible lengths in one comparison.
///
/// The output accumulates into a wipe-on-drop buffer sized so that it never
/// reallocates: `n` Base32 characters decode to exactly `n * 5 / 8` bytes, so a
/// rejected line can leave no intact copy of a partially decoded seed behind in
/// a stale allocation. The five-bit bit accumulator is wiped on every exit for
/// the same reason.
fn decode_totp_secret_line(line: &str) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    if line.is_empty() || line.len() > MAX_TOTP_SECRET_BASE32_CHARS {
        return Err(ApplicationError::InvalidInput);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(line.len() * 5 / 8));
    let mut buffer = 0_u16;
    let mut bits = 0_u8;
    for byte in line.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'2'..=b'7' => byte - b'2' + 26,
            _ => {
                buffer.zeroize();
                bits.zeroize();
                return Err(ApplicationError::InvalidInput);
            }
        };
        buffer = (buffer << 5) | u16::from(digit);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1_u16 << bits) - 1;
        }
    }
    let rejected = output.is_empty()
        || (bits != 0 && buffer != 0)
        || encode_totp_secret_base32(&output).as_str() != line;
    buffer.zeroize();
    bits.zeroize();
    if rejected {
        return Err(ApplicationError::InvalidInput);
    }
    Ok(output)
}

/// Re-encode decoded seed bytes as canonical unpadded RFC 4648 Base32.
///
/// This exists only to decide canonicality inside
/// [`decode_totp_secret_line`], so its result is a wipe-on-drop buffer that
/// never leaves the comparison.
fn encode_totp_secret_base32(value: &[u8]) -> Zeroizing<String> {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = Zeroizing::new(String::with_capacity((value.len() * 8).div_ceil(5)));
    let mut buffer = 0_u16;
    let mut bits = 0_u8;
    for byte in value {
        buffer = (buffer << 8) | u16::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
            buffer &= (1_u16 << bits) - 1;
        }
    }
    if bits != 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    buffer.zeroize();
    bits.zeroize();
    output
}

fn replacement_totp_document(
    current: &ItemDocument,
    input: TotpConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::TotpSeed(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    validate_totp_algorithm(&input.algorithm)?;
    let digits = parse_totp_digits_line(&input.digits)?;
    let period = parse_totp_period_line(&input.period)?;
    // Decoded last so that on the success path the owned seed bytes are handed
    // straight to the zeroizing record with no fallible step in between; on
    // every rejection path the wipe-on-drop buffer is dropped instead.
    let secret = decode_totp_secret_line(&input.secret)?;
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        // Every `TOTP_SEED_V1` field is authored by this form. Unlike the
        // database credential of VLT-PM37, the schema has no lease or other
        // issuance-only attribute, so nothing is inherited from the base
        // candidate and nothing resets to a static default.
        AnyRecord::TotpSeed(TotpSeed {
            label: input.label.into_inner(),
            issuer: input.issuer.map(Zeroizing::into_inner),
            secret: secret.into_inner(),
            algorithm: input.algorithm.into_inner(),
            digits,
            period,
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

impl OpaqueConflictMergePreparationV1 {
    /// Record a host-side prompt or entropy failure before exposing it.
    pub fn record_audited_host_failure(
        self,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let audited =
            self.publish_audited_failure(ApplicationError::InvalidInput, local_state_store)?;
        Ok(audited.into_parts().0)
    }

    /// Complete the authored opaque-record merge and its atomic event.
    pub fn complete_audited(
        self,
        input: OpaqueConflictMergeInputV1,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        let document =
            match replacement_opaque_document(&self.base, input, self.failure_audit.wall_time_ms) {
                Ok(document) => document,
                Err(error) => return self.publish_audited_failure(error, local_state_store),
            };
        let active = self.session.merge_item_conflict(
            document,
            self.failure_audit.wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
    }

    fn publish_audited_failure(
        self,
        error: ApplicationError,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.session.finish_audited_access(
            AuditActionV1::ItemConflictMerge,
            Some(self.item_id),
            None,
            self.failure_audit.wall_time_ms,
            self.failure_audit.randomness,
            local_state_store,
            Err(error),
        )
    }
}

/// Decode one already-bounded opaque-record payload line from lowercase hex.
///
/// Hexadecimal is the transport for this ceremony because an opaque payload is
/// arbitrary binary — canonical CBOR of a schema this product cannot read — and
/// a terminal line has to carry it losslessly. Two characters spell one byte:
///
/// ```text
///   characters  a  1  0  0  4  2
///   nibbles     1010 0001 0000 0000 0100 0010
///   bytes       0xa1      0x00      0x42
/// ```
///
/// Requiring lowercase and an even length makes the encoding canonical by
/// construction: every byte string has exactly one spelling, so unlike the
/// Base32 of `decode_totp_secret_line` there are no unused trailing bits to
/// check and no re-encode comparison to make. Canonicality still has to be
/// decided one level down, for the CBOR these bytes spell, and
/// [`canonical_opaque_payload`] does that.
///
/// The output accumulates into a wipe-on-drop buffer sized so that it never
/// reallocates — `n` hex characters decode to exactly `n / 2` bytes — so a
/// rejected line can leave no intact copy of a partially decoded payload behind
/// in a stale allocation. The pending high nibble is wiped on every exit for the
/// same reason.
fn decode_opaque_payload_line(line: &str) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    if line.is_empty() || line.len() > MAX_OPAQUE_PAYLOAD_HEX_CHARS || !line.len().is_multiple_of(2)
    {
        return Err(ApplicationError::InvalidInput);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(line.len() / 2));
    let mut high = 0_u8;
    let mut have_high = false;
    for byte in line.bytes() {
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => {
                high.zeroize();
                return Err(ApplicationError::InvalidInput);
            }
        };
        if have_high {
            output.push((high << 4) | nibble);
            have_high = false;
        } else {
            high = nibble;
            have_high = true;
        }
    }
    // The even-length check above already guarantees this, but stating it as a
    // fact the reader can check keeps the invariant local to the loop it
    // constrains.
    debug_assert!(!have_high, "an even-length line leaves no pending nibble");
    high.zeroize();
    Ok(output)
}

/// Require that decoded payload bytes are already canonical CBOR under one
/// inherited content type, and return them back as the accepted payload.
///
/// The check is a round trip through the same two functions the storage codec
/// uses: `encode_opaque` wraps the typed bytes in the base's content type,
/// which rejects anything that is not decodable CBOR or that carries trailing
/// bytes, and `decode_record` returns the canonical re-encoding of what was
/// wrapped. Requiring that re-encoding to equal the typed bytes is what makes
/// one stored payload have exactly one accepted spelling, and it is also the
/// property the codec needs, since the codec will re-encode this value again
/// every time the revision is sealed.
///
/// A round trip that does not come back as an opaque record means the content
/// type was one of the six first-party schemas, which cannot happen for a base
/// that decoded as opaque in the first place. Refusing it anyway makes it
/// structurally impossible to author a first-party record — a login, say —
/// through the one command that performs no schema validation.
fn canonical_opaque_payload(
    content_type: &str,
    typed: &[u8],
) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
    let wire = Zeroizing::new(
        encode_opaque(content_type, typed).map_err(|_| ApplicationError::InvalidInput)?,
    );
    let mut round_tripped = decode_record(&wire).map_err(|_| ApplicationError::InvalidInput)?;
    // The payload is moved out into wipe-on-drop ownership before anything can
    // return, and whatever remains — including a wrong-variant record's secret
    // fields on the refused path — is wiped unconditionally.
    let accepted = match &mut round_tripped {
        AnyRecord::Opaque {
            content_type: round_tripped_type,
            payload_bytes,
        } if round_tripped_type == content_type => {
            Some(Zeroizing::new(core::mem::take(payload_bytes)))
        }
        _ => None,
    };
    round_tripped.zeroize();
    let accepted = accepted.ok_or(ApplicationError::InternalInvariant)?;
    if accepted.as_slice() != typed {
        return Err(ApplicationError::InvalidInput);
    }
    Ok(accepted)
}

fn replacement_opaque_document(
    current: &ItemDocument,
    input: OpaqueConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::Opaque { content_type, .. } = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    let typed = decode_opaque_payload_line(&input.payload)?;
    let payload_bytes = canonical_opaque_payload(content_type, &typed)?;
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        // The content type is inherited from the base rather than authored. An
        // item's schema is immutable across its whole history: the document
        // validator requires the record's content type to equal the document
        // schema, and `merge_item_conflict` requires that schema to equal every
        // retained live candidate's. Exactly one field of this record is
        // therefore authored, which is the mirror image of VLT-PM38's TOTP
        // merge, where every field was.
        AnyRecord::Opaque {
            content_type: content_type.clone(),
            payload_bytes: payload_bytes.into_inner(),
        },
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

fn replacement_secure_note_document(
    current: &ItemDocument,
    input: SecureNoteConflictMergeInputV1,
    updated_at_ms: u64,
) -> Result<ItemDocument, ApplicationError> {
    let AnyRecord::SecureNote(_) = current.payload() else {
        return Err(ApplicationError::InternalInvariant);
    };
    ItemDocument::new(
        current.id(),
        current.schema().clone(),
        current.created_at_ms(),
        updated_at_ms.max(current.updated_at_ms()),
        current.favorite().clone(),
        current.collection_ids().clone(),
        current.tags().clone(),
        AnyRecord::SecureNote(SecureNote {
            title: input.title.into_inner(),
            body: input.body.into_inner(),
        }),
        current.attachments().clone(),
    )
    .map_err(|_| ApplicationError::InvalidInput)
}

/// One secret-free historical item revision projection.
#[derive(Clone, PartialEq, Eq)]
pub struct ItemHistoryViewV1 {
    revision_id: RevisionId,
    redacted_item: Option<RedactedItemView>,
    causal_parent_count: usize,
    advisory_time_ms: u64,
}

impl ItemHistoryViewV1 {
    fn from_candidate(candidate: &ItemCandidate) -> Result<Self, ApplicationError> {
        let (redacted_item, advisory_time_ms) = match candidate.state() {
            ItemState::Live(document) => (
                Some(
                    RedactedItemView::from_document(document)
                        .map_err(|_| ApplicationError::InternalInvariant)?,
                ),
                document.updated_at_ms(),
            ),
            ItemState::Tombstone(tombstone) => (None, tombstone.deleted_at_ms),
        };
        Ok(Self {
            revision_id: candidate.revision_id(),
            redacted_item,
            causal_parent_count: candidate.causal_parents().len(),
            advisory_time_ms,
        })
    }

    /// Return the exact encrypted revision object identity.
    pub const fn revision_id(&self) -> RevisionId {
        self.revision_id
    }

    /// Borrow safe live metadata, or `None` when this revision is a tombstone.
    pub const fn redacted_item(&self) -> Option<&RedactedItemView> {
        self.redacted_item.as_ref()
    }

    /// Return whether this historical revision is a deletion marker.
    pub const fn is_deleted(&self) -> bool {
        self.redacted_item.is_none()
    }

    /// Return the number of direct causal parents named by this revision.
    pub const fn causal_parent_count(&self) -> usize {
        self.causal_parent_count
    }

    /// Return the document-update or tombstone-deletion advisory time.
    pub const fn advisory_time_ms(&self) -> u64 {
        self.advisory_time_ms
    }
}

impl Debug for ItemHistoryViewV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ItemHistoryViewV1")
            .field("revision_id", &"<redacted>")
            .field("is_deleted", &self.is_deleted())
            .field("causal_parent_count", &self.causal_parent_count)
            .field("advisory_time_ms", &self.advisory_time_ms)
            .finish_non_exhaustive()
    }
}

struct CurrentCatalogV1 {
    items: BTreeMap<ItemId, Vec<ItemCandidate>>,
    candidate_count: usize,
}

impl CurrentCatalogV1 {
    fn item_count(&self) -> usize {
        self.items.len()
    }

    const fn candidate_count(&self) -> usize {
        self.candidate_count
    }

    fn conflicted_item_count(&self) -> usize {
        self.items
            .values()
            .filter(|candidates| candidates.len() > 1)
            .count()
    }
}

/// One authenticated active-vault session with live keys and a verified
/// repository view.
///
/// Dropping the session wipes its application keys and owner/device secrets.
/// The repository owns a separate wipe-on-drop verifier key set.
pub struct UnlockedVaultV1 {
    active: ActiveStateV1,
    report: OpenReport,
    current_catalog: CurrentCatalogV1,
    search: SearchProjectionV1,
    _keys: V1Keys,
    _local_secret: LocalSecretV1,
    _repository: Box<dyn ApplicationRepository>,
}

impl UnlockedVaultV1 {
    pub(crate) const fn active_state(&self) -> &ActiveStateV1 {
        &self.active
    }

    pub(crate) const fn bootstrap_locator(&self) -> BootstrapLocator {
        self.active.bootstrap_locator()
    }

    /// Return the authenticated vault identity.
    pub const fn vault_id(&self) -> VaultId {
        self.active.vault_id()
    }

    /// Return the authenticated local device identity.
    pub const fn device_id(&self) -> DeviceId {
        self.active.device_id()
    }

    /// Borrow the durable local head pins used to anchor this open.
    pub const fn local_pins(&self) -> &PinnedHeads {
        self.active.pinned_heads()
    }

    /// Return whether this vault has entered its durable operation-audit epoch.
    pub const fn audit_enabled(&self) -> bool {
        self.active.audit_event_head().is_some()
    }

    /// Begin the durable signed operation-audit epoch for a pre-audit vault.
    ///
    /// The unlocked session is consumed and the successful genesis event is
    /// published through the crash-resumable audit-only journal before the
    /// next owner state is returned. An already activated vault fails closed.
    /// Hosts must expose this transition only after all authenticated access
    /// and mutation paths can continue the epoch or fail closed.
    pub fn activate_audit_epoch(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        activate_audit_epoch(
            &self.active,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Borrow the complete payload-free verified repository report.
    pub const fn open_report(&self) -> &OpenReport {
        &self.report
    }

    /// Return the number of distinct current item identities without exposing
    /// any identity or item metadata.
    pub fn item_count(&self) -> usize {
        self.current_catalog.item_count()
    }

    /// Return the number of retained current revision candidates across all
    /// items. A value larger than [`Self::item_count`] indicates conflicts.
    pub const fn candidate_count(&self) -> usize {
        self.current_catalog.candidate_count()
    }

    /// Return how many current items retain more than one revision candidate.
    pub fn conflicted_item_count(&self) -> usize {
        self.current_catalog.conflicted_item_count()
    }

    /// Re-verify the complete reachable vault and return aggregate counts.
    ///
    /// This repeats repository discovery relative to durable local pins,
    /// checks the local writer counter/catalog anchor, walks complete verified
    /// ancestry from every head, and decrypts every distinct catalog and
    /// catalog-referenced revision. It returns no identities or item metadata.
    pub fn audit_verify(&self) -> Result<crate::AuditVerificationV1, ApplicationError> {
        crate::audit::audit_verify(&self.active, &self._keys, self._repository.as_ref())
    }

    /// Re-verify the complete reachable vault and release its aggregate report
    /// only after the verification-access event and next owner state are
    /// durable.
    ///
    /// A verification failure is recorded as a failed authenticated attempt
    /// when the repository remains able to publish the event. Publication
    /// failure supersedes and withholds both the report and original error.
    pub fn audited_verify(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<crate::AuditVerificationV1>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self.audit_verify();
        self.finish_audited_access(
            AuditActionV1::VaultVerify,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return newest-first redacted audit history after logging this access.
    ///
    /// The limit must be between one and
    /// [`crate::MAX_AUDIT_HISTORY_LIMIT`], inclusive. The session is consumed
    /// and a successful vault-scoped `AuditRead` event is made durable first.
    /// The application then re-verifies and projects the complete chain from
    /// that newly advanced owner state, so the returned bounded view includes
    /// the event that authorized it without recursively auditing itself.
    pub fn audited_audit_history(
        self,
        limit: usize,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Vec<crate::AuditEventViewV1>>, ApplicationError> {
        self.require_audit_epoch()?;
        if limit == 0 || limit > crate::MAX_AUDIT_HISTORY_LIMIT {
            return Err(ApplicationError::BoundExceeded);
        }
        let active = publish_audited_access(
            &self.active,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            AuditActionV1::AuditRead,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
        )?;
        let operation =
            crate::audit::audit_history(&active, &self._keys, self._repository.as_ref(), limit);
        Ok(crate::AuditedAccessResultV1::new(active, operation))
    }

    /// Return one redacted event selected by trace after logging this access.
    ///
    /// The successful `AuditRead` access event is published before the exact
    /// newly advanced chain is re-verified. `None` is a successful redacted
    /// lookup with no matching trace. The returned view never contains vault,
    /// device, repository, provider, path, signature, or secret payload data.
    pub fn audited_audit_event(
        self,
        trace_id: OperationId,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Option<crate::AuditEventViewV1>>, ApplicationError>
    {
        self.require_audit_epoch()?;
        let active = publish_audited_access(
            &self.active,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            AuditActionV1::AuditRead,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
        )?;
        let operation =
            crate::audit::audit_event(&active, &self._keys, self._repository.as_ref(), trace_id);
        Ok(crate::AuditedAccessResultV1::new(active, operation))
    }

    /// Build one canonical authenticated encrypted snapshot for host persistence.
    ///
    /// The passphrase must be collected separately from the live vault
    /// passphrase. The host supplies fresh salt/nonce randomness and chooses the
    /// destination only after this method returns. Every current live,
    /// tombstone, and conflicted candidate is included; local private state,
    /// provider credentials, pins, and search projections are excluded.
    pub fn export_portable_with_passphrase(
        &self,
        exact_bootstrap: &[u8],
        passphrase: Zeroizing<Vec<u8>>,
        policy: crate::PortableExportPolicyV1,
        randomness: crate::PortableExportRandomnessV1,
    ) -> Result<crate::PortableExportArtifactV1, ApplicationError> {
        crate::export::export_portable_with_passphrase(
            &self.current_catalog.items,
            &self.active,
            exact_bootstrap,
            passphrase,
            policy,
            randomness,
        )
    }

    /// Build and release one encrypted portable artifact only after its export
    /// access event and next owner state are durable.
    ///
    /// The separately collected passphrase and export randomness retain their
    /// existing owned wipe-on-drop behavior. Invalid export inputs are audited
    /// as failed attempts; audit-publication failure withholds the encrypted
    /// artifact and original operation error.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_export_portable_with_passphrase(
        self,
        exact_bootstrap: &[u8],
        passphrase: Zeroizing<Vec<u8>>,
        policy: crate::PortableExportPolicyV1,
        export_randomness: crate::PortableExportRandomnessV1,
        wall_time_ms: u64,
        audit_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<crate::PortableExportArtifactV1>, ApplicationError>
    {
        self.require_audit_epoch()?;
        let operation = self.export_portable_with_passphrase(
            exact_bootstrap,
            passphrase,
            policy,
            export_randomness,
        );
        self.finish_audited_access(
            AuditActionV1::PortableExport,
            None,
            None,
            wall_time_ms,
            audit_randomness,
            local_state_store,
            operation,
        )
    }

    /// Durably record a host-side portable-export input failure.
    ///
    /// The caller reserves the wall time and complete audit randomness before
    /// authentication, then uses this boundary if later collection of the
    /// distinct export passphrase fails. No partial passphrase or destination
    /// crosses into the event. The session is consumed and the closed failure
    /// becomes observable only after its audit-only commit is durable.
    pub fn record_audited_portable_export_host_failure(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let audited = self.finish_audited_access(
            AuditActionV1::PortableExport,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Err::<(), _>(ApplicationError::InvalidInput),
        )?;
        Ok(audited.into_parts().0)
    }

    /// Re-wrap the vault root key under a newly collected master passphrase.
    ///
    /// VLT-PM43. This is the whole of §14.8's "password rotation rewraps the
    /// VRK without re-encrypting every item body": it reads no item, opens no
    /// catalog, and publishes no repository object. It replaces one 32-byte
    /// ciphertext in one signed bootstrap record and moves the owner state's
    /// pin to it.
    ///
    /// The session is consumed because its `bootstrap_id` pin is stale the
    /// moment the rotation is durable. Its *keys* remain perfectly valid — the
    /// root key did not change — which is exactly why the operation is
    /// constant-time in the size of the vault.
    ///
    /// `current_passphrase` must be the one this session was opened with; see
    /// [`crate::prepare_passphrase_rotation`] for why it is collected again
    /// rather than retained. A vault whose audit epoch is active must use
    /// [`Self::audited_rotate_passphrase`] instead, so that the event is
    /// durable before the effect.
    #[allow(clippy::too_many_arguments)]
    pub fn rotate_passphrase(
        self,
        exact_bootstrap: &[u8],
        current_passphrase: &Zeroizing<Vec<u8>>,
        new_passphrase: &Zeroizing<Vec<u8>>,
        policy: crate::PassphraseRotationPolicyV1,
        randomness: crate::PassphraseRotationRandomnessV1,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let prepared = crate::rotate::prepare_passphrase_rotation(
            &self.active,
            exact_bootstrap,
            current_passphrase,
            new_passphrase,
            policy,
            &randomness,
            &self._local_secret,
        )?;
        crate::rotate::commit_passphrase_rotation(
            &self.active,
            prepared,
            local_state_store,
            bootstrap_store,
        )
    }

    /// Rotate the master passphrase only after the rotation event is durable.
    ///
    /// # Ordering
    ///
    /// ```text
    ///   prepare (pure)  ──▶  publish audit event  ──▶  durable rotation
    /// ```
    ///
    /// The audit event is published *before* the effect, per VLT-PM00 §3.9, and
    /// through the same crash-resumable audit-only journal every other audited
    /// ceremony uses. A preparation failure — a mismatched root key, an
    /// out-of-bounds Argon2id policy — is recorded as a failed attempt and then
    /// returned. An audit-publication failure supersedes and withholds the
    /// original error, because an unaudited rotation is worse than a reported
    /// one.
    ///
    /// The audit publication advances the owner state, so the durable rotation
    /// below is committed against *that* state rather than the one this session
    /// was opened with. [`crate::PreparedPassphraseRotationV1`] carries no
    /// owner state precisely so that this rebasing needs no special handling.
    ///
    /// The event carries no item, revision, salt, KDF parameter, generation
    /// number, or bootstrap identifier.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_rotate_passphrase(
        self,
        exact_bootstrap: &[u8],
        current_passphrase: &Zeroizing<Vec<u8>>,
        new_passphrase: &Zeroizing<Vec<u8>>,
        policy: crate::PassphraseRotationPolicyV1,
        randomness: crate::PassphraseRotationRandomnessV1,
        wall_time_ms: u64,
        audit_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let prepared = crate::rotate::prepare_passphrase_rotation(
            &self.active,
            exact_bootstrap,
            current_passphrase,
            new_passphrase,
            policy,
            &randomness,
            &self._local_secret,
        );
        let (audited_active, prepared) = self
            .finish_audited_access(
                AuditActionV1::PassphraseRotate,
                None,
                None,
                wall_time_ms,
                audit_randomness,
                local_state_store,
                prepared,
            )?
            .into_parts();
        crate::rotate::commit_passphrase_rotation(
            &audited_active,
            prepared?,
            local_state_store,
            bootstrap_store,
        )
    }

    /// Durably record a host-side passphrase-rotation input failure.
    ///
    /// The caller reserves the wall time and complete audit randomness before
    /// authentication, then uses this boundary if collecting or confirming the
    /// new passphrase fails outside the application. No partial passphrase and
    /// no confirmation state crosses into the event.
    pub fn record_audited_passphrase_rotation_host_failure(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let audited = self.finish_audited_access(
            AuditActionV1::PassphraseRotate,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Err::<(), _>(ApplicationError::InvalidInput),
        )?;
        Ok(audited.into_parts().0)
    }

    /// Durably record a host-side portable-import input failure.
    ///
    /// This boundary is used after an active-epoch target unlock when artifact
    /// reading, passphrase collection, no-write opening, or entropy collection
    /// fails outside the application. No source path, artifact bytes, partial
    /// passphrase, or target item identity enters the itemless event.
    pub fn record_audited_portable_import_host_failure(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let audited = self.finish_audited_access(
            AuditActionV1::PortableImport,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Err::<(), _>(ApplicationError::InvalidInput),
        )?;
        Ok(audited.into_parts().0)
    }

    /// Durably record a host-side portable-restore verification input failure.
    ///
    /// This boundary is used after an active-epoch target unlock when artifact
    /// reading, passphrase collection, no-write opening, or expectation
    /// preparation fails outside the target comparison. No source path,
    /// artifact bytes, partial passphrase, target identity, or mismatch detail
    /// enters the itemless event.
    pub fn record_audited_portable_restore_verify_host_failure(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let audited = self.finish_audited_access(
            AuditActionV1::PortableRestoreVerify,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Err::<(), _>(ApplicationError::InvalidInput),
        )?;
        Ok(audited.into_parts().0)
    }

    /// Run authenticated low-resolution diagnostics and release the coarse
    /// report only after its access event and next owner state are durable.
    ///
    /// Diagnostic health is report data rather than operation failure: a
    /// completed diagnostic is a successful access even when its coarse state
    /// reports an unavailable or unhealthy dependency. If audit publication
    /// fails, no diagnostic report is released.
    pub fn audited_doctor(
        self,
        local_state_store: &dyn LocalStateStore,
        bootstrap_store: &dyn BootstrapStore,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
    ) -> Result<crate::AuditedAccessResultV1<crate::VaultDoctorReportV1>, ApplicationError> {
        self.require_audit_epoch()?;
        let access = crate::VaultAccessV1::Unlocked(Box::new(self));
        let report = access.doctor(local_state_store, bootstrap_store);
        let session = access
            .into_unlocked()
            .map_err(|_| ApplicationError::InternalInvariant)?;
        session.finish_audited_access(
            AuditActionV1::VaultDiagnose,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Ok(report),
        )
    }

    /// Consume an authenticated portable snapshot into this untouched target
    /// vault and return the resulting durable active owner state.
    ///
    /// The target must still be the empty generation-zero vault. Every source
    /// item and retained live, tombstone, or conflicted candidate receives a
    /// new target item/revision/object identity and is encrypted by the target
    /// vault's independent keys. Source causal-parent identities are not
    /// copied. The complete import is one crash-resumable publication, and the
    /// session, opaque snapshot, and owned randomness are consumed on every
    /// return path.
    pub fn import_opened_portable_snapshot(
        self,
        snapshot: crate::OpenedPortableSnapshotV1,
        wall_time_ms: u64,
        randomness: PortableImportRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        import_opened_portable_snapshot(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            snapshot,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Atomically import an opened snapshot or publish its failed target-side attempt.
    ///
    /// Success uses the mutation's own `PortableImport` event in the same
    /// repository publication as every re-identified candidate. A target
    /// precondition, identity, bound, or repository-preparation failure instead
    /// advances the audit-only chain before the closed error is returned.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_import_opened_portable_snapshot(
        self,
        snapshot: crate::OpenedPortableSnapshotV1,
        wall_time_ms: u64,
        import_randomness: PortableImportRandomnessV1,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = import_opened_portable_snapshot(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            snapshot,
            wall_time_ms,
            import_randomness,
            local_state_store,
        );
        match operation {
            Ok(active) => Ok(active),
            Err(error) => {
                self.finish_audited_access(
                    AuditActionV1::PortableImport,
                    None,
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err::<(), _>(error),
                )?
                .into_operation()?;
                Err(ApplicationError::InternalInvariant)
            }
        }
    }

    /// Independently compare a reopened target with its authenticated source semantics.
    ///
    /// The expectation is opaque to hosts and normalizes only identities that
    /// cross-vault import must replace. Match or mismatch publishes a dedicated
    /// itemless audit event before the aggregate report or closed error can be
    /// observed. Source/target identity overlap, retained causal parents, and
    /// any schema, timestamp, deletion, grouping, CRDT, or record-value change
    /// fail with the same integrity class.
    pub fn audited_verify_portable_restore(
        self,
        expectation: crate::PortableRestoreExpectationV1,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<crate::PortableRestoreVerificationV1>, ApplicationError>
    {
        self.require_audit_epoch()?;
        let operation =
            expectation.verify_target(self.active.vault_id(), &self.current_catalog.items);
        self.finish_audited_access(
            AuditActionV1::PortableRestoreVerify,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return the ordinary redacted view for one unambiguous live item.
    ///
    /// A missing item and a current tombstone both return `None`. Multiple
    /// retained candidates fail closed with [`ApplicationError::ConflictRequired`]
    /// rather than selecting a winner or exposing only part of the conflict.
    pub fn get_item(&self, item_id: ItemId) -> Result<Option<RedactedItemView>, ApplicationError> {
        let Some(candidates) = self.current_catalog.items.get(&item_id) else {
            return Ok(None);
        };
        project_current_item(candidates)
    }

    /// Return one current redacted item only after its access event and next
    /// owner state are durable.
    ///
    /// Missing and tombstoned items become the closed `NotFound` operation
    /// result. A successful event binds the exact selected live revision;
    /// conflicts are recorded as failed attempts. Event-publication failure
    /// releases none of those results.
    pub fn audited_get_item(
        self,
        item_id: ItemId,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<RedactedItemView>, ApplicationError> {
        self.require_audit_epoch()?;
        let (operation, selected_revision) = match self.current_catalog.items.get(&item_id) {
            None => (Err(ApplicationError::NotFound), None),
            Some(candidates) => {
                let operation = project_current_item(candidates)
                    .and_then(|view| view.ok_or(ApplicationError::NotFound));
                let selected_revision = operation.is_ok().then(|| candidates[0].revision_id());
                (operation, selected_revision)
            }
        };
        self.finish_audited_access(
            AuditActionV1::ItemRead,
            Some(item_id),
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return the exact sole current live revision for optimistic mutation.
    ///
    /// A missing item and a current tombstone both return `None`. Multiple
    /// retained candidates fail closed with [`ApplicationError::ConflictRequired`].
    /// The revision identity is an application capability for a later
    /// compare-and-swap mutation and is not an ordinary display value.
    pub fn current_item_revision(
        &self,
        item_id: ItemId,
    ) -> Result<Option<RevisionId>, ApplicationError> {
        let Some(candidates) = self.current_catalog.items.get(&item_id) else {
            return Ok(None);
        };
        let [candidate] = candidates.as_slice() else {
            return Err(ApplicationError::ConflictRequired);
        };
        Ok(matches!(candidate.state(), ItemState::Live(_)).then_some(candidate.revision_id()))
    }

    /// Return the exact sole current live revision capability only after its
    /// item-read event and next owner state are durable.
    ///
    /// Missing and tombstoned items become audited `NotFound` failures, while
    /// current conflicts become audited `ConflictRequired` failures. A
    /// successful event binds the exact returned revision. Publication failure
    /// withholds both the capability and original operation error.
    pub fn audited_current_item_revision(
        self,
        item_id: ItemId,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<RevisionId>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self
            .current_item_revision(item_id)
            .and_then(|revision| revision.ok_or(ApplicationError::NotFound));
        let selected_revision = operation.as_ref().ok().copied();
        self.finish_audited_access(
            AuditActionV1::ItemRead,
            Some(item_id),
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return every unambiguous live item as an ordinary redacted view.
    ///
    /// Views are ordered by exact item-ID bytes. A current conflict aborts the
    /// complete read with [`ApplicationError::ConflictRequired`]; no partial
    /// list is returned and every retained candidate remains in the session.
    pub fn list_items(&self) -> Result<Vec<RedactedItemView>, ApplicationError> {
        let mut views = Vec::with_capacity(self.current_catalog.items.len());
        for candidates in self.current_catalog.items.values() {
            if let Some(view) = project_current_item(candidates)? {
                views.push(view);
            }
        }
        Ok(views)
    }

    /// Return the redacted item list only after its audit event and next owner
    /// state are durable.
    ///
    /// This consumes the session so callers cannot keep reading from stale
    /// pins after the audit-only commit advances the repository. Both a
    /// successful list and a post-authentication list failure are recorded;
    /// the latter is returned inside [`crate::AuditedAccessResultV1`] with the
    /// durable next state. If event publication fails, this method returns that
    /// storage or integrity error directly and releases no list or original
    /// operation failure. Pre-audit sessions reject the boundary until an
    /// explicit migration epoch has been installed.
    pub fn audited_list_items(
        self,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Vec<RedactedItemView>>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self.list_items();
        self.finish_audited_access(
            AuditActionV1::ItemList,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Search unambiguous live items using only approved redacted metadata.
    ///
    /// The owned query is wiped on every return path. It must contain 1–256
    /// UTF-8 bytes and no control characters. Results optionally require
    /// membership in one explicit collection and are ordered by normalized
    /// display title, schema, then exact item-ID bytes. Any current conflict
    /// aborts the complete search without returning partial results.
    pub fn search_items(
        &self,
        query: Zeroizing<String>,
        collection: Option<CollectionId>,
        limit: usize,
    ) -> Result<Vec<RedactedItemView>, ApplicationError> {
        if self.current_catalog.conflicted_item_count() != 0 {
            return Err(ApplicationError::ConflictRequired);
        }
        self.search
            .search(query, collection, limit, &self.current_catalog.items)
    }

    /// Return a redacted search result only after its access event and next
    /// owner state are durable.
    ///
    /// Invalid queries, bounds, and current conflicts are recorded as failed
    /// post-authentication attempts. The owned query remains wipe-on-drop, and
    /// event-publication failure releases no search result or original error.
    pub fn audited_search_items(
        self,
        query: Zeroizing<String>,
        collection: Option<CollectionId>,
        limit: usize,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Vec<RedactedItemView>>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self.search_items(query, collection, limit);
        self.finish_audited_access(
            AuditActionV1::ItemSearch,
            None,
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return how many unambiguous live items are held in the wipe-on-lock
    /// search projection without exposing item identities or indexed text.
    pub fn search_item_count(&self) -> usize {
        self.search.len()
    }

    /// Return bounded secret-free history for one item across every current
    /// repository head.
    ///
    /// Traversal is newest ancestry depth first. Commits at the same depth and
    /// revisions in the same catalog are ordered by exact object ID. Revisions
    /// reached through more than one head are returned once. `limit` must be
    /// between 1 and [`MAX_ITEM_HISTORY_LIMIT`], inclusive.
    pub fn item_history(
        &self,
        item_id: ItemId,
        limit: usize,
    ) -> Result<Vec<ItemHistoryViewV1>, ApplicationError> {
        materialize_item_history_candidates(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            item_id,
            limit,
        )?
        .iter()
        .map(ItemHistoryViewV1::from_candidate)
        .collect()
    }

    /// Return one item's redacted history only after its access event and next
    /// owner state are durable.
    ///
    /// Invalid bounds and repository/integrity failures are represented only
    /// after a failed history-access event can be published. Publication
    /// failure supersedes and withholds the history result.
    pub fn audited_item_history(
        self,
        item_id: ItemId,
        limit: usize,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Vec<ItemHistoryViewV1>>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self.item_history(item_id, limit);
        self.finish_audited_access(
            AuditActionV1::ItemHistoryRead,
            Some(item_id),
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Return every retained current candidate for one conflicted item as a
    /// deterministic secret-free view.
    ///
    /// Candidates are ordered by exact revision ID. A missing item returns
    /// `NotFound`; an item with fewer than two current candidates returns
    /// `ConflictRequired`. No candidate is selected or discarded.
    pub fn conflict_candidates(
        &self,
        item_id: ItemId,
    ) -> Result<Vec<ItemHistoryViewV1>, ApplicationError> {
        let candidates = self
            .current_catalog
            .items
            .get(&item_id)
            .ok_or(ApplicationError::NotFound)?;
        if candidates.len() < 2 {
            return Err(ApplicationError::ConflictRequired);
        }
        candidates
            .iter()
            .map(ItemHistoryViewV1::from_candidate)
            .collect()
    }

    /// Return current redacted conflict candidates only after their history
    /// access event and next owner state are durable.
    ///
    /// Missing and unconflicted items remain closed operation failures, but
    /// become observable only after their failed access event is durable.
    pub fn audited_conflict_candidates(
        self,
        item_id: ItemId,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Vec<ItemHistoryViewV1>>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self.conflict_candidates(item_id);
        self.finish_audited_access(
            AuditActionV1::ItemHistoryRead,
            Some(item_id),
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    fn require_audit_epoch(&self) -> Result<(), ApplicationError> {
        self.active
            .audit_event_head()
            .map(|_| ())
            .ok_or(ApplicationError::InvalidInput)
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_audited_access<T>(
        self,
        action: AuditActionV1,
        item_id: Option<ItemId>,
        selected_revision: Option<RevisionId>,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
        operation: Result<T, ApplicationError>,
    ) -> Result<crate::AuditedAccessResultV1<T>, ApplicationError> {
        let outcome = if operation.is_ok() {
            AuditOutcomeV1::Succeeded
        } else {
            AuditOutcomeV1::Failed
        };
        self.finish_audited_access_with_outcome(
            action,
            outcome,
            item_id,
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_audited_access_with_outcome<T>(
        self,
        action: AuditActionV1,
        outcome: AuditOutcomeV1,
        item_id: Option<ItemId>,
        selected_revision: Option<RevisionId>,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
        operation: Result<T, ApplicationError>,
    ) -> Result<crate::AuditedAccessResultV1<T>, ApplicationError> {
        if operation.is_ok() != (outcome == AuditOutcomeV1::Succeeded) {
            return Err(ApplicationError::InternalInvariant);
        }
        let active = publish_audited_access(
            &self.active,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            action,
            outcome,
            item_id,
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
        )?;
        Ok(crate::AuditedAccessResultV1::new(active, operation))
    }

    /// Explicitly reveal one reachable live revision inside an owned
    /// wipe-on-drop wrapper.
    ///
    /// The exact revision must appear in a catalog reachable within the hard
    /// history bound from a current head. Tombstones return `InvalidInput` and
    /// unreachable revisions return `NotFound`. The wrapper deliberately has
    /// no `Debug`, `Display`, or `Clone` implementation.
    pub fn reveal_item_revision(
        &self,
        selected_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let selected = find_reachable_historical_candidate(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            selected_revision,
        )?;
        let ItemState::Live(document) = selected.state() else {
            return Err(ApplicationError::InvalidInput);
        };
        Ok(Zeroizing::new(document.as_ref().clone()))
    }

    /// Reveal one exact live revision for an item only after its item-read
    /// event and next owner state are durable.
    ///
    /// The caller supplies the item identity explicitly so the event can bind
    /// the complete resource. A mismatched, unreachable, or tombstoned
    /// revision is retained as a closed failed attempt without releasing a
    /// document. The successful owned document remains wipe-on-drop.
    pub fn audited_reveal_item_revision(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<Zeroizing<ItemDocument>>, ApplicationError> {
        self.require_audit_epoch()?;
        let operation = self
            .reveal_item_revision(selected_revision)
            .and_then(|document| {
                if document.id() != item_id {
                    return Err(ApplicationError::InvalidInput);
                }
                Ok(document)
            });
        let event_revision = operation.is_ok().then_some(selected_revision);
        self.finish_audited_access(
            AuditActionV1::ItemRead,
            Some(item_id),
            event_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Select and authorize disclosure of one secret-bearing field from one
    /// exact reachable live revision.
    ///
    /// Policy is checked before repository traversal. The returned value is
    /// owned, non-printable, non-cloneable, and wipe-on-drop. The host remains
    /// responsible for its controlling-TTY facts, warning output, and secure
    /// clipboard ownership/clear behavior.
    pub fn reveal_item_revision_field(
        &self,
        selected_revision: RevisionId,
        field: SecretFieldV1,
        intent: SecretDisclosureIntentV1,
    ) -> Result<RevealedSecretV1, ApplicationError> {
        intent.authorize()?;
        let document = self.reveal_item_revision(selected_revision)?;
        crate::disclosure::select_secret(document.payload(), field)
    }

    /// Authorize and reveal one secret field only after its item-read event and
    /// next owner state are durable.
    ///
    /// A refused disclosure ceremony publishes a `Denied` event without
    /// traversing the selected revision. Authorized selection failures publish
    /// `Failed`; success binds the exact item and revision and holds the owned
    /// non-printable secret until publication completes.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_reveal_item_revision_field(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        field: SecretFieldV1,
        intent: SecretDisclosureIntentV1,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<RevealedSecretV1>, ApplicationError> {
        self.require_audit_epoch()?;
        let (outcome, event_revision, operation) = match intent.authorize() {
            Err(error) => (AuditOutcomeV1::Denied, None, Err(error)),
            Ok(()) => match self.reveal_item_revision(selected_revision) {
                Err(error) => (AuditOutcomeV1::Failed, None, Err(error)),
                Ok(document) if document.id() != item_id => (
                    AuditOutcomeV1::Failed,
                    None,
                    Err(ApplicationError::InvalidInput),
                ),
                Ok(document) => {
                    let operation = crate::disclosure::select_secret(document.payload(), field);
                    let outcome = if operation.is_ok() {
                        AuditOutcomeV1::Succeeded
                    } else {
                        AuditOutcomeV1::Failed
                    };
                    (outcome, Some(selected_revision), operation)
                }
            },
        };
        self.finish_audited_access_with_outcome(
            AuditActionV1::ItemRead,
            outcome,
            Some(item_id),
            event_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Authorize and reveal one secret field from one exact current conflict
    /// candidate only after its item-read event and next owner state are
    /// durable.
    ///
    /// Refusal publishes `Denied` without traversing the candidate. Missing or
    /// unconflicted items and revisions outside the current conflict set
    /// publish `Failed` without accepting a historical revision as a current
    /// candidate. Once membership is authenticated, tombstone and field
    /// selection failures bind the exact revision; success holds the owned
    /// non-printable secret until publication completes.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_reveal_conflict_candidate_field(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        field: SecretFieldV1,
        intent: SecretDisclosureIntentV1,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<RevealedSecretV1>, ApplicationError> {
        self.require_audit_epoch()?;
        let (outcome, event_revision, operation) = match intent.authorize() {
            Err(error) => (AuditOutcomeV1::Denied, None, Err(error)),
            Ok(()) => match self.conflict_candidate_validation(item_id, selected_revision) {
                Err(error) => (AuditOutcomeV1::Failed, None, Err(error)),
                Ok(()) => {
                    let operation =
                        self.reveal_item_revision(selected_revision)
                            .and_then(|document| {
                                if document.id() != item_id {
                                    return Err(ApplicationError::InvalidInput);
                                }
                                crate::disclosure::select_secret(document.payload(), field)
                            });
                    let outcome = if operation.is_ok() {
                        AuditOutcomeV1::Succeeded
                    } else {
                        AuditOutcomeV1::Failed
                    };
                    (outcome, Some(selected_revision), operation)
                }
            },
        };
        self.finish_audited_access_with_outcome(
            AuditActionV1::ItemRead,
            outcome,
            Some(item_id),
            event_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    fn conflict_candidate_validation(
        &self,
        item_id: ItemId,
        selected_revision: RevisionId,
    ) -> Result<(), ApplicationError> {
        let candidates = self
            .current_catalog
            .items
            .get(&item_id)
            .ok_or(ApplicationError::NotFound)?;
        if candidates.len() < 2 {
            return Err(ApplicationError::ConflictRequired);
        }
        if !candidates
            .iter()
            .any(|candidate| candidate.revision_id() == selected_revision)
        {
            return Err(ApplicationError::NotFound);
        }
        Ok(())
    }

    /// Select and authorize one secret field from the sole current live item
    /// only after its item-read event and next owner state are durable.
    ///
    /// The current revision capability never crosses the application boundary.
    /// A refused ceremony publishes `Denied` without traversing the item;
    /// missing, tombstoned, conflicted, and schema-mismatched selections
    /// publish `Failed`; success binds the exact current revision and retains
    /// the owned non-printable secret until publication completes.
    pub fn audited_reveal_current_item_field(
        self,
        item_id: ItemId,
        field: SecretFieldV1,
        intent: SecretDisclosureIntentV1,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<RevealedSecretV1>, ApplicationError> {
        self.require_audit_epoch()?;
        let (outcome, selected_revision, operation) = match intent.authorize() {
            Err(error) => (AuditOutcomeV1::Denied, None, Err(error)),
            Ok(()) => match self.current_catalog.items.get(&item_id) {
                None => (
                    AuditOutcomeV1::Failed,
                    None,
                    Err(ApplicationError::NotFound),
                ),
                Some(candidates) => match candidates.as_slice() {
                    [candidate] => match candidate.state() {
                        ItemState::Tombstone(_) => (
                            AuditOutcomeV1::Failed,
                            None,
                            Err(ApplicationError::NotFound),
                        ),
                        ItemState::Live(document) => {
                            let revision = candidate.revision_id();
                            let operation =
                                crate::disclosure::select_secret(document.payload(), field);
                            let outcome = if operation.is_ok() {
                                AuditOutcomeV1::Succeeded
                            } else {
                                AuditOutcomeV1::Failed
                            };
                            (outcome, Some(revision), operation)
                        }
                    },
                    _ => (
                        AuditOutcomeV1::Failed,
                        None,
                        Err(ApplicationError::ConflictRequired),
                    ),
                },
            },
        };
        self.finish_audited_access_with_outcome(
            AuditActionV1::ItemRead,
            outcome,
            Some(item_id),
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
            operation,
        )
    }

    /// Add one new item through the exact crash-resumable publication state
    /// machine and return the resulting durable active owner state.
    ///
    /// The session is consumed so a successful caller cannot keep using stale
    /// pins, catalog contents, or search state. The document and randomness
    /// are owned and wiped on every return path. Hosts must reopen a new
    /// session after success or recover the durable pending journal after an
    /// interrupted provider/local effect.
    pub fn add_item(
        self,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: AddItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        add_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Durably record a host-side failure while collecting a new item.
    ///
    /// The caller must reserve the complete add-item randomness before
    /// authentication. This boundary derives and binds the item identity from
    /// that owned block, then wipes the unused mutation material. No partially
    /// collected record field crosses the boundary. The session is consumed,
    /// and the closed failure becomes observable only after its audit-only
    /// commit is durable.
    pub fn record_audited_item_create_host_failure(
        self,
        add_randomness: AddItemRandomnessV1,
        wall_time_ms: u64,
        randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.require_audit_epoch()?;
        let item_id = add_randomness.item_id();
        let audited = self.finish_audited_access(
            AuditActionV1::ItemCreate,
            Some(item_id),
            None,
            wall_time_ms,
            randomness,
            local_state_store,
            Err::<(), _>(ApplicationError::InvalidInput),
        )?;
        Ok(audited.into_parts().0)
    }

    /// Replace the sole expected current live revision and return the resulting
    /// durable active owner state.
    ///
    /// The replacement preserves item identity, content schema, and creation
    /// time. Its new revision directly names `expected_revision` as its causal
    /// parent. A missing item returns `NotFound`; a stale, tombstoned, or
    /// conflicted current candidate returns `ConflictRequired`. The session and
    /// all owned mutation inputs are consumed on every return path.
    pub fn replace_item(
        self,
        expected_revision: RevisionId,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: ReplaceItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        replace_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            expected_revision,
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Validate one current login edit without releasing its revision or body.
    ///
    /// Missing and tombstoned items return `NotFound`; current conflicts return
    /// `ConflictRequired`; non-login records and logins with more than one URL
    /// return `Unsupported`. A successful opaque preparation owns this session,
    /// the exact current revision, and the wipe-on-drop current document.
    pub fn prepare_login_edit(
        self,
        item_id: ItemId,
    ) -> Result<LoginEditPreparationV1, ApplicationError> {
        let (operation, _) = self.login_edit_precondition(item_id);
        let (expected_revision, current) = operation?;
        Ok(LoginEditPreparationV1 {
            session: self,
            item_id,
            expected_revision,
            current,
            failure_audit: None,
        })
    }

    /// Validate one login edit or durably publish its failed precondition.
    pub fn prepare_audited_login_edit(
        self,
        item_id: ItemId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedLoginEditPreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        let (operation, selected_revision) = self.login_edit_precondition(item_id);
        match operation {
            Ok((expected_revision, current)) => Ok(AuditedLoginEditPreparationV1::Ready(Box::new(
                LoginEditPreparationV1 {
                    session: self,
                    item_id,
                    expected_revision,
                    current,
                    failure_audit: Some(LoginEditFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    }),
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemUpdate,
                    Some(item_id),
                    selected_revision,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedLoginEditPreparationV1::Failed(Box::new(failure))),
        }
    }

    fn login_edit_precondition(&self, item_id: ItemId) -> LoginEditPreconditionV1 {
        let expected_revision = match self.current_item_revision(item_id) {
            Ok(Some(revision)) => revision,
            Ok(None) => return (Err(ApplicationError::NotFound), None),
            Err(error) => return (Err(error), None),
        };
        let current = match self.reveal_item_revision(expected_revision) {
            Ok(current) => current,
            Err(error) => return (Err(error), Some(expected_revision)),
        };
        let AnyRecord::Login(_) = current.payload() else {
            return (Err(ApplicationError::Unsupported), Some(expected_revision));
        };
        (Ok((expected_revision, current)), Some(expected_revision))
    }

    /// Delete the sole expected current live revision by publishing a causal
    /// tombstone and return the resulting durable active owner state.
    ///
    /// A revision absent from the current catalog returns `NotFound`; a
    /// conflicted or already-tombstoned target returns `ConflictRequired`.
    /// Advisory deletion and commit times are supplied separately and do not
    /// establish causality. The session and randomness are consumed on every
    /// return path.
    pub fn delete_item(
        self,
        expected_revision: RevisionId,
        deleted_at_ms: u64,
        wall_time_ms: u64,
        randomness: DeleteItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        delete_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            expected_revision,
            deleted_at_ms,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Delete the sole current live revision selected internally by item ID.
    ///
    /// This keeps the exact optimistic-mutation revision capability inside the
    /// application boundary. Missing and tombstoned items return `NotFound`;
    /// current conflicts return `ConflictRequired`. When an audit epoch is
    /// active, the resulting `ItemDelete` event binds the internally selected
    /// revision atomically with the causal tombstone publication.
    pub fn delete_current_item(
        self,
        item_id: ItemId,
        deleted_at_ms: u64,
        wall_time_ms: u64,
        randomness: DeleteItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let expected_revision = self
            .current_item_revision(item_id)?
            .ok_or(ApplicationError::NotFound)?;
        self.delete_item(
            expected_revision,
            deleted_at_ms,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Delete the sole current live revision, recording failed preconditions.
    ///
    /// Successful deletion uses the ordinary atomic mutation publication, so
    /// the `ItemDelete` event and tombstone share one causal commit. Missing,
    /// tombstoned, and conflicted items instead publish a failed `ItemDelete`
    /// event before the closed operation error becomes observable. Audit-event
    /// publication failure supersedes and withholds that operation error.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_delete_current_item(
        self,
        item_id: ItemId,
        deleted_at_ms: u64,
        wall_time_ms: u64,
        mutation_randomness: DeleteItemRandomnessV1,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.require_audit_epoch()?;
        let expected_revision = self
            .current_item_revision(item_id)
            .and_then(|revision| revision.ok_or(ApplicationError::NotFound));
        match expected_revision {
            Ok(expected_revision) => {
                let active = self.delete_item(
                    expected_revision,
                    deleted_at_ms,
                    wall_time_ms,
                    mutation_randomness,
                    local_state_store,
                )?;
                Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
            }
            Err(error) => self.finish_audited_access(
                AuditActionV1::ItemDelete,
                Some(item_id),
                None,
                wall_time_ms,
                failure_randomness,
                local_state_store,
                Err(error),
            ),
        }
    }

    /// Restore one reachable historical live revision as a new current live
    /// revision and return the resulting durable active owner state.
    ///
    /// The selected revision must be reachable within the hard history bound,
    /// belong to an item with exactly one current candidate, and differ from
    /// that current revision. Tombstones cannot be restored. The new revision
    /// copies the selected live document and names only the selected revision
    /// as its direct causal parent; repository heads are never rewound.
    pub fn restore_item(
        self,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        randomness: RestoreItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        let selected = find_reachable_historical_candidate(
            &self._keys,
            self._repository.as_ref(),
            &self.report,
            self.active.vault_id(),
            selected_revision,
        )?;
        restore_item(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            selected,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Restore one bounded historical live revision bound to an item ID.
    ///
    /// Selection, item binding, tombstone rejection, current-conflict checks,
    /// and same-revision rejection all remain inside the application boundary.
    /// `history_limit` has the same strict bounds and deterministic ancestry
    /// semantics as [`Self::item_history`].
    #[allow(clippy::too_many_arguments)]
    pub fn restore_item_for_item(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        history_limit: usize,
        wall_time_ms: u64,
        randomness: RestoreItemRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        self.restore_selection_validation(item_id, selected_revision, history_limit)
            .0?;
        self.restore_item(
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Restore one item-bound historical revision after durable audit outcome.
    ///
    /// A successful `ItemRestore` event and its new live revision share the
    /// causal mutation publication. Invalid bounds, missing or cross-item
    /// selectors, tombstone selectors, same-revision requests, and current
    /// conflicts publish a failed event before their closed operation error is
    /// observable. Audit publication failure supersedes and withholds that
    /// operation error.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_restore_item_for_item(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        history_limit: usize,
        wall_time_ms: u64,
        mutation_randomness: RestoreItemRandomnessV1,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.require_audit_epoch()?;
        let (validation, event_revision) =
            self.restore_selection_validation(item_id, selected_revision, history_limit);
        match validation {
            Ok(()) => {
                let active = self.restore_item(
                    selected_revision,
                    wall_time_ms,
                    mutation_randomness,
                    local_state_store,
                )?;
                Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
            }
            Err(error) => self.finish_audited_access(
                AuditActionV1::ItemRestore,
                Some(item_id),
                event_revision,
                wall_time_ms,
                failure_randomness,
                local_state_store,
                Err(error),
            ),
        }
    }

    fn restore_selection_validation(
        &self,
        item_id: ItemId,
        selected_revision: RevisionId,
        history_limit: usize,
    ) -> (Result<(), ApplicationError>, Option<RevisionId>) {
        let history = match self.item_history(item_id, history_limit) {
            Ok(history) => history,
            Err(error) => return (Err(error), None),
        };
        let Some(selected) = history
            .into_iter()
            .find(|candidate| candidate.revision_id() == selected_revision)
        else {
            return (Err(ApplicationError::NotFound), None);
        };
        let event_revision = Some(selected_revision);
        if selected.is_deleted() {
            return (Err(ApplicationError::InvalidInput), event_revision);
        }
        let Some(current) = self.current_catalog.items.get(&item_id) else {
            return (Err(ApplicationError::NotFound), event_revision);
        };
        let [current] = current.as_slice() else {
            return (Err(ApplicationError::ConflictRequired), event_revision);
        };
        if current.revision_id() == selected_revision {
            return (Err(ApplicationError::InvalidInput), event_revision);
        }
        (Ok(()), event_revision)
    }

    /// Resolve one current conflict by choosing an existing authenticated
    /// candidate and publishing it as a new current revision.
    ///
    /// The selected revision must be one of at least two current candidates.
    /// The resolution revision copies its complete live document or tombstone
    /// and names every retained current candidate as a direct causal parent.
    /// This consumes the session and never deletes the losing immutable bytes.
    pub fn resolve_item_conflict(
        self,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        resolve_item_conflict(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            selected_revision,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }

    /// Resolve one item-bound current conflict after a durable audit outcome.
    ///
    /// The selected revision must belong to the named item's current conflict
    /// set. Success publishes the `ItemConflictResolve` event atomically with
    /// the new resolution revision. Missing items, unconflicted items, and
    /// missing or cross-item candidate selectors publish a failed event before
    /// their closed operation error becomes observable.
    #[allow(clippy::too_many_arguments)]
    pub fn audited_resolve_item_conflict_for_item(
        self,
        item_id: ItemId,
        selected_revision: RevisionId,
        wall_time_ms: u64,
        mutation_randomness: ResolveItemConflictRandomnessV1,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<crate::AuditedAccessResultV1<()>, ApplicationError> {
        self.require_audit_epoch()?;
        let (validation, event_revision) =
            self.conflict_resolution_validation(item_id, selected_revision);
        match validation {
            Ok(()) => {
                let active = self.resolve_item_conflict(
                    selected_revision,
                    wall_time_ms,
                    mutation_randomness,
                    local_state_store,
                )?;
                Ok(crate::AuditedAccessResultV1::new(active, Ok(())))
            }
            Err(error) => self.finish_audited_access(
                AuditActionV1::ItemConflictResolve,
                Some(item_id),
                event_revision,
                wall_time_ms,
                failure_randomness,
                local_state_store,
                Err(error),
            ),
        }
    }

    fn conflict_resolution_validation(
        &self,
        item_id: ItemId,
        selected_revision: RevisionId,
    ) -> (Result<(), ApplicationError>, Option<RevisionId>) {
        let Some(candidates) = self.current_catalog.items.get(&item_id) else {
            return (Err(ApplicationError::NotFound), None);
        };
        if candidates.len() < 2 {
            return (Err(ApplicationError::ConflictRequired), None);
        }
        if candidates
            .iter()
            .any(|candidate| candidate.revision_id() == selected_revision)
        {
            (Ok(()), Some(selected_revision))
        } else {
            (Err(ApplicationError::NotFound), None)
        }
    }

    /// Validate one exact current live login as the opaque metadata base for
    /// an authored conflict merge, or publish the failed precondition before
    /// releasing its closed error.
    ///
    /// The complete base document remains application-owned. Every retained
    /// live candidate must share its schema and creation time so completion can
    /// safely publish through the all-current-parent merge primitive.
    pub fn prepare_audited_login_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedLoginConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.login_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedLoginConflictMergePreparationV1::Ready(Box::new(
                LoginConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedLoginConflictMergePreparationV1::Failed(Box::new(failure))),
        }
    }

    fn login_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::Login(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live secure note as the opaque metadata base
    /// for an authored conflict merge, or publish the failed precondition.
    pub fn prepare_audited_secure_note_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedSecureNoteConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.secure_note_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedSecureNoteConflictMergePreparationV1::Ready(
                Box::new(SecureNoteConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                }),
            )),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| {
                    AuditedSecureNoteConflictMergePreparationV1::Failed(Box::new(failure))
                }),
        }
    }

    fn secure_note_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::SecureNote(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live payment card as the opaque metadata base
    /// for an authored conflict merge, or publish the failed precondition.
    pub fn prepare_audited_card_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedCardConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.card_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedCardConflictMergePreparationV1::Ready(Box::new(
                CardConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedCardConflictMergePreparationV1::Failed(Box::new(failure))),
        }
    }

    fn card_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::Card(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live API key as the opaque metadata base for
    /// an authored conflict merge, or publish the failed precondition.
    pub fn prepare_audited_api_key_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedApiKeyConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.api_key_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedApiKeyConflictMergePreparationV1::Ready(Box::new(
                ApiKeyConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedApiKeyConflictMergePreparationV1::Failed(Box::new(failure))),
        }
    }

    fn api_key_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::ApiKey(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live database credential as the opaque
    /// metadata base for an authored conflict merge, or publish the failed
    /// precondition.
    pub fn prepare_audited_database_credential_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedDatabaseCredentialConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.database_credential_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedDatabaseCredentialConflictMergePreparationV1::Ready(
                Box::new(DatabaseCredentialConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                }),
            )),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| {
                    AuditedDatabaseCredentialConflictMergePreparationV1::Failed(Box::new(failure))
                }),
        }
    }

    fn database_credential_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::DatabaseCredential(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live TOTP seed as the opaque metadata base
    /// for an authored conflict merge, or publish the failed precondition.
    pub fn prepare_audited_totp_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedTotpConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.totp_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedTotpConflictMergePreparationV1::Ready(Box::new(
                TotpConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedTotpConflictMergePreparationV1::Failed(Box::new(failure))),
        }
    }

    fn totp_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        let AnyRecord::TotpSeed(_) = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    /// Validate one exact current live opaque record as the metadata base for
    /// an authored conflict merge, or publish the failed precondition.
    pub fn prepare_audited_opaque_conflict_merge(
        self,
        item_id: ItemId,
        base_revision: RevisionId,
        wall_time_ms: u64,
        failure_randomness: AuditedAccessRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<AuditedOpaqueConflictMergePreparationV1, ApplicationError> {
        self.require_audit_epoch()?;
        match self.opaque_conflict_merge_precondition(item_id, base_revision) {
            Ok(base) => Ok(AuditedOpaqueConflictMergePreparationV1::Ready(Box::new(
                OpaqueConflictMergePreparationV1 {
                    session: self,
                    item_id,
                    base,
                    failure_audit: ConflictMergeFailureAuditV1 {
                        wall_time_ms,
                        randomness: failure_randomness,
                    },
                },
            ))),
            Err(error) => self
                .finish_audited_access(
                    AuditActionV1::ItemConflictMerge,
                    Some(item_id),
                    None,
                    wall_time_ms,
                    failure_randomness,
                    local_state_store,
                    Err(error),
                )
                .map(|failure| AuditedOpaqueConflictMergePreparationV1::Failed(Box::new(failure))),
        }
    }

    fn opaque_conflict_merge_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let base = self.conflict_merge_base_precondition(item_id, base_revision)?;
        // Every first-party schema has its own authored merge with its own
        // closed field rules, so an item this product does understand is
        // refused here rather than being routed through the one ceremony that
        // validates no fields.
        let AnyRecord::Opaque { .. } = base.payload() else {
            return Err(ApplicationError::Unsupported);
        };
        Ok(base)
    }

    fn conflict_merge_base_precondition(
        &self,
        item_id: ItemId,
        base_revision: RevisionId,
    ) -> Result<Zeroizing<ItemDocument>, ApplicationError> {
        let candidates = self
            .current_catalog
            .items
            .get(&item_id)
            .ok_or(ApplicationError::NotFound)?;
        if candidates.len() < 2 {
            return Err(ApplicationError::ConflictRequired);
        }
        let selected = candidates
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .ok_or(ApplicationError::NotFound)?;
        let ItemState::Live(base) = selected.state() else {
            return Err(ApplicationError::InvalidInput);
        };
        if base.id() != item_id {
            return Err(ApplicationError::InvalidInput);
        }
        for candidate in candidates {
            if let ItemState::Live(current) = candidate.state() {
                if current.id() != item_id
                    || current.schema() != base.schema()
                    || current.created_at_ms() != base.created_at_ms()
                {
                    return Err(ApplicationError::InvalidInput);
                }
            }
        }
        Ok(Zeroizing::new((**base).clone()))
    }

    /// Resolve one current conflict with a complete caller-authored document.
    ///
    /// The document must name an item with at least two current candidates and
    /// preserve the schema and creation time of every retained live candidate.
    /// At least one live candidate is required. The new revision names the
    /// complete current conflict set as direct causal parents, consumes the
    /// session and owned secret-bearing document, and never deletes immutable
    /// candidate bytes.
    pub fn merge_item_conflict(
        self,
        document: ItemDocument,
        wall_time_ms: u64,
        randomness: ResolveItemConflictRandomnessV1,
        local_state_store: &dyn LocalStateStore,
    ) -> Result<ActiveStateV1, ApplicationError> {
        merge_item_conflict(
            &self.active,
            &self.report,
            &self.current_catalog.items,
            &self._keys,
            &self._local_secret,
            self._repository.as_ref(),
            document,
            wall_time_ms,
            randomness,
            local_state_store,
        )
    }
}

fn project_current_item(
    candidates: &[ItemCandidate],
) -> Result<Option<RedactedItemView>, ApplicationError> {
    let [candidate] = candidates else {
        return Err(ApplicationError::ConflictRequired);
    };
    match candidate.state() {
        ItemState::Live(document) => RedactedItemView::from_document(document)
            .map(Some)
            .map_err(|_| ApplicationError::InternalInvariant),
        ItemState::Tombstone(_) => Ok(None),
    }
}

impl Debug for UnlockedVaultV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnlockedVaultV1")
            .field("local_pin_count", &self.active.pinned_heads().len())
            .field("verified_head_count", &self.report.heads().len())
            .field("item_count", &self.current_catalog.item_count())
            .field("candidate_count", &self.current_catalog.candidate_count())
            .field(
                "conflicted_item_count",
                &self.current_catalog.conflicted_item_count(),
            )
            .field("search_item_count", &self.search.len())
            .finish_non_exhaustive()
    }
}

/// Authenticated-reopen one stable `Active` vault from injected byte stores.
///
/// This slice deliberately accepts only `Active`; callers must complete a
/// `PreparedInit` or recover a `PendingPublication` before invoking it. The
/// latest bootstrap must exactly match the locally pinned signed generation,
/// the passphrase must authenticate its root wrap, all private seeds must
/// reproduce pinned public identities, and the repository must open relative
/// to non-empty local pins.
pub fn open_active_vault(
    passphrase: Zeroizing<Vec<u8>>,
    locator: BootstrapLocator,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    repository_factory: &dyn ApplicationRepositoryFactory,
) -> Result<UnlockedVaultV1, ApplicationError> {
    let exact_state = local_state_store
        .load(locator)
        .map_err(map_local_state_store)?
        .ok_or(ApplicationError::NotInitialized)?;
    let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_state)? else {
        return Err(ApplicationError::InvalidInput);
    };
    if active.bootstrap_locator() != locator {
        return Err(ApplicationError::IntegrityFailure);
    }

    let exact_bootstrap = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    let material = unlock_active_material(passphrase, &active, &exact_bootstrap)?;
    let repository = repository_factory
        .connect(material.repository_address, Box::new(material.verifier))
        .map_err(map_repository)?;
    repository.initialize().map_err(map_repository)?;
    let report = repository
        .open(active.pinned_heads())
        .map_err(map_repository)?;
    if report.fresh_device_unanchored() || report.heads().is_empty() {
        return Err(ApplicationError::IntegrityFailure);
    }
    let current_catalog = materialize_current_catalog(
        &material.keys,
        repository.as_ref(),
        &report,
        active.vault_id(),
    )?;
    let search = SearchProjectionV1::build(&current_catalog.items)?;

    Ok(UnlockedVaultV1 {
        active,
        report,
        current_catalog,
        search,
        _keys: material.keys,
        _local_secret: material.local_secret,
        _repository: repository,
    })
}

fn materialize_current_catalog(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
) -> Result<CurrentCatalogV1, ApplicationError> {
    let mut materialized = BTreeMap::<ItemId, BTreeMap<RevisionId, ItemCandidate>>::new();
    let mut seen_catalogs = BTreeSet::new();

    for head_id in report.heads().iter().copied() {
        let commit = repository.read_commit(head_id).map_err(map_repository)?;
        if commit.id() != head_id || commit.vault_id() != vault_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let catalog_id = commit.catalog_root();
        if !seen_catalogs.insert(catalog_id) {
            continue;
        }
        let catalog_object = repository.read_object(catalog_id).map_err(map_repository)?;
        if catalog_object.id() != catalog_id {
            return Err(ApplicationError::IntegrityFailure);
        }
        let catalog_plaintext = open_object(keys, ObjectKind::Catalog, catalog_object.frame())?;
        let catalog = CatalogV1::decode(&catalog_plaintext)?;

        for (item_id, revision_ids) in catalog.entries() {
            if !materialized.contains_key(item_id) && materialized.len() == MAX_CATALOG_ENTRIES {
                return Err(ApplicationError::IntegrityFailure);
            }
            let candidates = materialized.entry(*item_id).or_default();
            for revision_id in revision_ids {
                if candidates.contains_key(revision_id) {
                    continue;
                }
                if candidates.len() == MAX_CANDIDATES_PER_ITEM {
                    return Err(ApplicationError::IntegrityFailure);
                }
                let candidate = read_candidate(keys, repository, *revision_id)?;
                if candidate.item_id() != *item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
                for parent_id in candidate.causal_parents() {
                    let parent = read_candidate(keys, repository, *parent_id)?;
                    if parent.item_id() != *item_id {
                        return Err(ApplicationError::IntegrityFailure);
                    }
                }
                candidates.insert(*revision_id, candidate);
            }
        }
    }

    let candidate_count = materialized.values().map(BTreeMap::len).sum();
    Ok(CurrentCatalogV1 {
        items: materialized
            .into_iter()
            .map(|(item_id, candidates)| (item_id, candidates.into_values().collect()))
            .collect(),
        candidate_count,
    })
}

pub(crate) fn read_candidate(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    revision_id: RevisionId,
) -> Result<ItemCandidate, ApplicationError> {
    let object_id = ObjectId::new(*revision_id.as_bytes());
    let revision_object = repository.read_object(object_id).map_err(map_repository)?;
    if revision_object.id() != object_id {
        return Err(ApplicationError::IntegrityFailure);
    }
    let revision_plaintext = open_object(keys, ObjectKind::ItemRevision, revision_object.frame())?;
    crate::decode_item_revision(revision_id, &revision_plaintext)
}

fn materialize_item_history_candidates(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
    item_id: ItemId,
    limit: usize,
) -> Result<Vec<ItemCandidate>, ApplicationError> {
    if limit == 0 {
        return Err(ApplicationError::InvalidInput);
    }
    if limit > MAX_ITEM_HISTORY_LIMIT {
        return Err(ApplicationError::BoundExceeded);
    }

    let mut histories = Vec::with_capacity(report.heads().len());
    for head_id in report.heads().iter().copied() {
        let history = repository.history(head_id, limit).map_err(map_repository)?;
        if history.first().map(|commit| commit.id()) != Some(head_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        histories.push(history);
    }

    let mut seen_commits = BTreeSet::new();
    let mut seen_catalogs = BTreeSet::new();
    let mut seen_revisions = BTreeSet::new();
    let mut candidates = Vec::new();

    for depth in 0..limit {
        let mut commits = histories
            .iter()
            .filter_map(|history| history.get(depth))
            .filter(|commit| !seen_commits.contains(&commit.id()))
            .collect::<Vec<_>>();
        commits.sort_unstable_by_key(|commit| commit.id());

        for commit in commits {
            if !seen_commits.insert(commit.id()) {
                continue;
            }
            if commit.vault_id() != vault_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let catalog_id = commit.catalog_root();
            if !seen_catalogs.insert(catalog_id) {
                continue;
            }
            let catalog_object = repository.read_object(catalog_id).map_err(map_repository)?;
            if catalog_object.id() != catalog_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let catalog_plaintext = open_object(keys, ObjectKind::Catalog, catalog_object.frame())?;
            let catalog = CatalogV1::decode(&catalog_plaintext)?;
            let Some(revision_ids) = catalog.entries().get(&item_id) else {
                continue;
            };

            for revision_id in revision_ids {
                if seen_revisions.contains(revision_id) {
                    continue;
                }
                let candidate = read_candidate(keys, repository, *revision_id)?;
                if candidate.item_id() != item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
                for parent_id in candidate.causal_parents() {
                    let parent = read_candidate(keys, repository, *parent_id)?;
                    if parent.item_id() != item_id {
                        return Err(ApplicationError::IntegrityFailure);
                    }
                }
                seen_revisions.insert(*revision_id);
                candidates.push(candidate);
                if candidates.len() == limit {
                    return Ok(candidates);
                }
            }
        }
    }

    Ok(candidates)
}

fn find_reachable_historical_candidate(
    keys: &V1Keys,
    repository: &dyn ApplicationRepository,
    report: &OpenReport,
    vault_id: VaultId,
    selected_revision: RevisionId,
) -> Result<ItemCandidate, ApplicationError> {
    let mut histories = Vec::with_capacity(report.heads().len());
    for head_id in report.heads().iter().copied() {
        let history = repository
            .history(head_id, MAX_ITEM_HISTORY_LIMIT)
            .map_err(map_repository)?;
        if history.first().map(|commit| commit.id()) != Some(head_id) {
            return Err(ApplicationError::IntegrityFailure);
        }
        histories.push(history);
    }

    let mut seen_commits = BTreeSet::new();
    let mut seen_catalogs = BTreeSet::new();
    for depth in 0..MAX_ITEM_HISTORY_LIMIT {
        let mut commits = histories
            .iter()
            .filter_map(|history| history.get(depth))
            .filter(|commit| !seen_commits.contains(&commit.id()))
            .collect::<Vec<_>>();
        commits.sort_unstable_by_key(|commit| commit.id());

        for commit in commits {
            if !seen_commits.insert(commit.id()) {
                continue;
            }
            if commit.vault_id() != vault_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let catalog_id = commit.catalog_root();
            if !seen_catalogs.insert(catalog_id) {
                continue;
            }
            let catalog_object = repository.read_object(catalog_id).map_err(map_repository)?;
            if catalog_object.id() != catalog_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            let catalog_plaintext = open_object(keys, ObjectKind::Catalog, catalog_object.frame())?;
            let catalog = CatalogV1::decode(&catalog_plaintext)?;
            let Some((item_id, _)) = catalog
                .entries()
                .iter()
                .find(|(_, revisions)| revisions.binary_search(&selected_revision).is_ok())
            else {
                continue;
            };

            let candidate = read_candidate(keys, repository, selected_revision)?;
            if candidate.item_id() != *item_id {
                return Err(ApplicationError::IntegrityFailure);
            }
            for parent_id in candidate.causal_parents() {
                let parent = read_candidate(keys, repository, *parent_id)?;
                if parent.item_id() != *item_id {
                    return Err(ApplicationError::IntegrityFailure);
                }
            }
            return Ok(candidate);
        }
    }

    Err(ApplicationError::NotFound)
}

/// Replay one exact durable `PendingPublication` and atomically advance it to
/// `Active` only after the repository returns the journal's expected pins.
///
/// Provider ambiguity leaves the exact journal untouched for another retry.
/// A concurrent local writer is accepted only when it installed the identical
/// intended `Active` bytes; every other winner fails closed.
pub fn recover_pending_publication(
    passphrase: Zeroizing<Vec<u8>>,
    locator: BootstrapLocator,
    local_state_store: &dyn LocalStateStore,
    bootstrap_store: &dyn BootstrapStore,
    repository_factory: &dyn ApplicationRepositoryFactory,
) -> Result<ActiveStateV1, ApplicationError> {
    let exact_pending = local_state_store
        .load(locator)
        .map_err(map_local_state_store)?
        .ok_or(ApplicationError::NotInitialized)?;
    let LocalVaultStateV1::PendingPublication {
        active,
        publication,
    } = LocalVaultStateV1::decode(&exact_pending)?
    else {
        return Err(ApplicationError::InvalidInput);
    };
    if active.bootstrap_locator() != locator {
        return Err(ApplicationError::IntegrityFailure);
    }

    let exact_bootstrap = bootstrap_store
        .load_latest(locator)
        .map_err(map_bootstrap_store)?
        .ok_or(ApplicationError::IntegrityFailure)?;
    let UnlockedActiveMaterial {
        repository_address,
        keys: _keys,
        local_secret: _local_secret,
        verifier,
    } = unlock_active_material(passphrase, &active, &exact_bootstrap)?;
    let repository = repository_factory
        .connect(repository_address, Box::new(verifier))
        .map_err(map_repository)?;
    repository.initialize().map_err(map_repository)?;
    let receipt = repository
        .publish(publication.publication(), publication.base_heads())
        .map_err(map_repository)?;
    if receipt.heads() != publication.expected_heads() {
        return Err(ApplicationError::IntegrityFailure);
    }

    let intended_active = active.after_publication(&publication)?;
    let exact_active = LocalVaultStateV1::Active(intended_active.clone()).encode()?;
    match local_state_store.compare_exchange(locator, Some(&exact_pending), &exact_active) {
        Ok(()) => Ok(intended_active),
        Err(LocalStateStoreError::ConcurrentHost) => {
            match local_state_store
                .load(locator)
                .map_err(map_local_state_store)?
            {
                Some(observed) if observed == exact_active => Ok(intended_active),
                _ => Err(ApplicationError::ConcurrentHost),
            }
        }
        Err(error) => Err(map_local_state_store(error)),
    }
}

fn map_bootstrap_store(error: BootstrapStoreError) -> ApplicationError {
    match error {
        BootstrapStoreError::Unavailable => ApplicationError::StorageUnavailable,
        BootstrapStoreError::Conflict | BootstrapStoreError::Corruption => {
            ApplicationError::IntegrityFailure
        }
    }
}

fn map_local_state_store(error: LocalStateStoreError) -> ApplicationError {
    match error {
        LocalStateStoreError::Unavailable => ApplicationError::StorageUnavailable,
        LocalStateStoreError::ConcurrentHost => ApplicationError::ConcurrentHost,
        LocalStateStoreError::Corruption => ApplicationError::IntegrityFailure,
    }
}

fn map_repository(error: ApplicationRepositoryError) -> ApplicationError {
    match error {
        ApplicationRepositoryError::NotInitialized => ApplicationError::NotInitialized,
        ApplicationRepositoryError::InvalidInput => ApplicationError::InvalidInput,
        ApplicationRepositoryError::BoundExceeded => ApplicationError::BoundExceeded,
        ApplicationRepositoryError::StorageUnavailable => ApplicationError::StorageUnavailable,
        ApplicationRepositoryError::IntegrityFailure => ApplicationError::IntegrityFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutation::{
        activate_audit_epoch_for_test, publish_audit_only_event_for_test,
        AUDIT_ONLY_TEST_RANDOM_BYTES,
    };
    use crate::{
        complete_generation_zero, decode_signed_audit_event, encode_item_revision,
        encode_signed_commit, prepare_generation_zero, seal_object, AuditedAccessRandomnessV1,
        CatalogV1, GenerationZeroPolicyV1, GenerationZeroRandomness, ObjectKind, ObjectRandomness,
        PassphraseRotationPolicyV1, PassphraseRotationRandomnessV1, PendingRotationV1,
        PublicationJournalV1, V1ApplicationRepositoryFactory, V1Keys, ADD_ITEM_RANDOM_BYTES,
        AUDITED_ACCESS_RANDOM_BYTES, DELETE_ITEM_RANDOM_BYTES, GENERATION_ZERO_RANDOM_BYTES,
        PASSPHRASE_ROTATION_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES,
        RESOLVE_ITEM_CONFLICT_RANDOM_BYTES, RESTORE_ITEM_RANDOM_BYTES,
    };
    use coding_adventures_canonical_cbor::{
        decode as decode_cbor, encode as encode_cbor, CborValue,
    };
    use coding_adventures_ed25519::{generate_keypair, sign};
    use coding_adventures_vault_pm_audit::{AuditActionV1, AuditOutcomeV1};
    use coding_adventures_vault_pm_domain::{
        ContentType, ItemDocument, ItemState, LwwRegister, ObservedSet, OperationId,
        RedactedRecordView, Tombstone,
    };
    use coding_adventures_vault_pm_format::{
        AnnouncementV1, BootstrapId, BootstrapV1, CommitV1, Signature,
    };
    use coding_adventures_vault_pm_storage::{
        FaultAction, FaultEffect, FaultInjectingObjectStore, InMemoryObjectStore, StoreOperation,
        VaultObjectStore,
    };
    use coding_adventures_vault_records::{AnyRecord, Login, SecureNote, LOGIN_V1, SECURE_NOTE_V1};
    use coding_adventures_zeroize::Zeroize;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    };

    struct FailingAuditRepository(ApplicationRepositoryError);

    impl ApplicationRepository for FailingAuditRepository {
        fn initialize(&self) -> Result<(), ApplicationRepositoryError> {
            Err(self.0)
        }

        fn open(&self, _pins: &PinnedHeads) -> Result<OpenReport, ApplicationRepositoryError> {
            Err(self.0)
        }

        fn publish(
            &self,
            _publication: coding_adventures_vault_pm_repository::Publication,
            _current_heads: &PinnedHeads,
        ) -> Result<
            coding_adventures_vault_pm_repository::PublicationReceipt,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }

        fn read_object(
            &self,
            _id: ObjectId,
        ) -> Result<coding_adventures_vault_pm_repository::VerifiedObject, ApplicationRepositoryError>
        {
            Err(self.0)
        }

        fn read_commit(
            &self,
            _id: ObjectId,
        ) -> Result<coding_adventures_vault_pm_repository::CommitSummary, ApplicationRepositoryError>
        {
            Err(self.0)
        }

        fn history(
            &self,
            _start: ObjectId,
            _limit: usize,
        ) -> Result<
            Vec<coding_adventures_vault_pm_repository::CommitSummary>,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }

        fn complete_history(
            &self,
            _start: ObjectId,
        ) -> Result<
            Vec<coding_adventures_vault_pm_repository::CommitSummary>,
            ApplicationRepositoryError,
        > {
            Err(self.0)
        }
    }

    #[derive(Default)]
    struct MemoryLocalStateStore(
        Mutex<Option<Vec<u8>>>,
        AtomicBool,
        Mutex<Option<(usize, LocalStateStoreError)>>,
        AtomicUsize,
    );

    impl MemoryLocalStateStore {
        fn with_state(state: Vec<u8>) -> Self {
            Self(
                Mutex::new(Some(state)),
                AtomicBool::new(false),
                Mutex::new(None),
                AtomicUsize::new(0),
            )
        }

        fn concurrent_winner_on_next_compare(&self) {
            self.1.store(true, Ordering::SeqCst);
        }

        fn fail_next_compare(&self, error: LocalStateStoreError) {
            self.fail_compare_after(0, error);
        }

        fn fail_compare_after(&self, successful_calls: usize, error: LocalStateStoreError) {
            let target = self
                .3
                .load(Ordering::SeqCst)
                .checked_add(successful_calls + 1)
                .unwrap();
            *self.2.lock().unwrap() = Some((target, error));
        }
    }

    impl LocalStateStore for MemoryLocalStateStore {
        fn load(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn compare_exchange(
            &self,
            _locator: BootstrapLocator,
            expected: Option<&[u8]>,
            replacement: &[u8],
        ) -> Result<(), LocalStateStoreError> {
            let call = self.3.fetch_add(1, Ordering::SeqCst) + 1;
            let mut state = self.0.lock().unwrap();
            if state.as_deref() != expected {
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            let scheduled_error = *self.2.lock().unwrap();
            if let Some((target, error)) = scheduled_error {
                if target == call {
                    self.2.lock().unwrap().take();
                    return Err(error);
                }
            }
            if self.1.swap(false, Ordering::SeqCst) {
                *state = Some(replacement.to_vec());
                return Err(LocalStateStoreError::ConcurrentHost);
            }
            *state = Some(replacement.to_vec());
            Ok(())
        }
    }

    /// An in-memory bootstrap store that keeps a real generation chain.
    ///
    /// The single-slot fixture this replaced could not express the state a
    /// passphrase rotation actually produces — two generations installed, one
    /// of them retired — so it could not have caught a rotation that left the
    /// retired wrap behind. `retired` retains superseded records exactly the
    /// way a filesystem adapter would, so a test can assert that
    /// `supersede_generation` really removed one.
    #[derive(Default)]
    struct MemoryBootstrapStore {
        latest: Mutex<Option<Vec<u8>>>,
        retired: Mutex<Vec<Vec<u8>>>,
    }

    impl MemoryBootstrapStore {
        fn with_latest(bytes: Vec<u8>) -> Self {
            Self {
                latest: Mutex::new(Some(bytes)),
                retired: Mutex::new(Vec::new()),
            }
        }

        fn latest_bytes(&self) -> Option<Vec<u8>> {
            self.latest.lock().unwrap().clone()
        }

        fn retired_bytes(&self) -> Vec<Vec<u8>> {
            self.retired.lock().unwrap().clone()
        }
    }

    impl BootstrapStore for MemoryBootstrapStore {
        fn load_latest(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
            Ok(self.latest_bytes())
        }

        fn put_generation(
            &self,
            _locator: BootstrapLocator,
            expected_previous: Option<BootstrapId>,
            exact_bootstrap: &[u8],
        ) -> Result<(), BootstrapStoreError> {
            let intended = coding_adventures_vault_pm_format::BootstrapV1::decode(exact_bootstrap)
                .map_err(|_| BootstrapStoreError::Corruption)?;
            if intended.previous_bootstrap != expected_previous {
                return Err(BootstrapStoreError::Conflict);
            }
            let mut latest = self.latest.lock().unwrap();
            match latest.as_ref() {
                Some(current) if current == exact_bootstrap => return Ok(()),
                Some(current) => {
                    let current = coding_adventures_vault_pm_format::BootstrapV1::decode(current)
                        .map_err(|_| BootstrapStoreError::Corruption)?;
                    let current_id = current.id().map_err(|_| BootstrapStoreError::Corruption)?;
                    if expected_previous != Some(current_id)
                        || intended.generation != current.generation + 1
                        || intended.vault_id != current.vault_id
                    {
                        return Err(BootstrapStoreError::Conflict);
                    }
                    self.retired.lock().unwrap().push(
                        current
                            .encode()
                            .map_err(|_| BootstrapStoreError::Corruption)?,
                    );
                }
                None => {
                    if expected_previous.is_some() || intended.generation != 0 {
                        return Err(BootstrapStoreError::Conflict);
                    }
                }
            }
            *latest = Some(exact_bootstrap.to_vec());
            Ok(())
        }

        fn supersede_generation(
            &self,
            _locator: BootstrapLocator,
            superseded: BootstrapId,
        ) -> Result<(), BootstrapStoreError> {
            if let Some(current) = self.latest_bytes() {
                if coding_adventures_vault_pm_format::BootstrapV1::decode(&current)
                    .ok()
                    .and_then(|bootstrap| bootstrap.id().ok())
                    == Some(superseded)
                {
                    return Err(BootstrapStoreError::Conflict);
                }
            }
            self.retired.lock().unwrap().retain(|bytes| {
                coding_adventures_vault_pm_format::BootstrapV1::decode(bytes)
                    .ok()
                    .and_then(|bootstrap| bootstrap.id().ok())
                    != Some(superseded)
            });
            Ok(())
        }
    }

    fn generation_zero_bytes() -> [u8; GENERATION_ZERO_RANDOM_BYTES] {
        let mut bytes = [0; GENERATION_ZERO_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(7);
        }
        bytes
    }

    fn randomness() -> GenerationZeroRandomness {
        GenerationZeroRandomness::new(generation_zero_bytes())
    }

    fn replace_top_level_version(encoded: &[u8], version: u64) -> Vec<u8> {
        let CborValue::Map(mut fields) = decode_cbor(encoded).unwrap() else {
            panic!("fixture must be a CBOR map")
        };
        let mut replaced = false;
        for (key, value) in &mut fields {
            if key == &CborValue::Unsigned(1) {
                *value = CborValue::Unsigned(version);
                replaced = true;
                break;
            }
        }
        assert!(replaced);
        encode_cbor(&CborValue::Map(fields))
    }

    fn take_cbor_field(fields: &mut Vec<(CborValue, CborValue)>, key: u64) -> CborValue {
        let index = fields
            .iter()
            .position(|(candidate, _)| candidate == &CborValue::Unsigned(key))
            .unwrap();
        fields.remove(index).1
    }

    fn replace_cbor_field(fields: &mut [(CborValue, CborValue)], key: u64, replacement: CborValue) {
        let (_, value) = fields
            .iter_mut()
            .find(|(candidate, _)| candidate == &CborValue::Unsigned(key))
            .unwrap();
        *value = replacement;
    }

    fn refresh_portable_snapshot_hash(fields: &mut [(CborValue, CborValue)]) {
        let CborValue::Bytes(mut bootstrap) = fields
            .iter()
            .find(|(key, _)| key == &CborValue::Unsigned(2))
            .map(|(_, value)| value.clone())
            .unwrap()
        else {
            panic!()
        };
        let entries = fields
            .iter()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| encode_cbor(value))
            .unwrap();
        let hash = crate::export::snapshot_hash(&bootstrap, &entries).unwrap();
        replace_cbor_field(fields, 5, CborValue::Bytes(hash.to_vec()));
        bootstrap.zeroize();
    }

    fn authenticate_portable_snapshot(
        fields: Vec<(CborValue, CborValue)>,
        passphrase: &[u8],
        randomness: u8,
    ) -> Vec<u8> {
        let mut plaintext = encode_cbor(&CborValue::Map(fields));
        let artifact = crate::export::encrypt_portable_for_test(
            &plaintext,
            Zeroizing::new(passphrase.to_vec()),
            crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            crate::PortableExportRandomnessV1::new(
                [randomness; crate::PORTABLE_EXPORT_RANDOM_BYTES],
            ),
        );
        plaintext.zeroize();
        artifact.into_bytes()
    }

    fn add_item_randomness(seed: u8) -> AddItemRandomnessV1 {
        let mut bytes = [0; ADD_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(seed);
        }
        AddItemRandomnessV1::new(bytes)
    }

    fn replace_item_randomness(seed: u8) -> ReplaceItemRandomnessV1 {
        let mut bytes = [0; REPLACE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(seed);
        }
        ReplaceItemRandomnessV1::new(bytes)
    }

    fn delete_item_randomness(seed: u8) -> DeleteItemRandomnessV1 {
        let mut bytes = [0; DELETE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(31).wrapping_add(seed);
        }
        DeleteItemRandomnessV1::new(bytes)
    }

    fn restore_item_randomness(seed: u8) -> RestoreItemRandomnessV1 {
        let mut bytes = [0; RESTORE_ITEM_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(37).wrapping_add(seed);
        }
        RestoreItemRandomnessV1::new(bytes)
    }

    fn resolve_item_conflict_randomness(seed: u8) -> ResolveItemConflictRandomnessV1 {
        let mut bytes = [0; RESOLVE_ITEM_CONFLICT_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(41).wrapping_add(seed);
        }
        ResolveItemConflictRandomnessV1::new(bytes)
    }

    fn audited_access_randomness(seed: u8) -> AuditedAccessRandomnessV1 {
        let mut bytes = [0; AUDITED_ACCESS_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(43).wrapping_add(seed);
        }
        AuditedAccessRandomnessV1::new(bytes)
    }

    fn audited_access_trace(seed: u8) -> OperationId {
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(43).wrapping_add(seed);
        }
        OperationId::new(bytes)
    }

    fn latest_audit_facts(
        session: &UnlockedVaultV1,
    ) -> (
        AuditActionV1,
        AuditOutcomeV1,
        Option<ItemId>,
        Option<RevisionId>,
    ) {
        let event_head = session.active.audit_event_head().unwrap();
        let object = session._repository.read_object(event_head).unwrap();
        let plaintext =
            open_object(&session._keys, ObjectKind::AuditEvent, object.frame()).unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        (
            event.event().action(),
            event.event().outcome(),
            event.event().item_id(),
            event.event().selected_revision(),
        )
    }

    fn new_login_document(item_id: ItemId, title: &str, password: &str) -> ItemDocument {
        login_document_with_times(item_id, title, password, 300, 300)
    }

    fn rich_login_document(item_id: ItemId, title: &str, password: &str) -> ItemDocument {
        ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            300,
            300,
            LwwRegister::new(false, 300, OperationId::new([0x71; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: title.to_owned(),
                username: "multi@example.test".to_owned(),
                password: password.to_owned(),
                urls: vec![
                    "https://one.example.test".to_owned(),
                    "https://two.example.test".to_owned(),
                ],
                notes: Some("old private notes".to_owned()),
            }),
            ObservedSet::new(),
        )
        .unwrap()
    }

    fn login_document_with_times(
        item_id: ItemId,
        title: &str,
        password: &str,
        created_at_ms: u64,
        updated_at_ms: u64,
    ) -> ItemDocument {
        ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            created_at_ms,
            updated_at_ms,
            LwwRegister::new(false, updated_at_ms, OperationId::new([0x71; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: title.to_owned(),
                username: "new-user@example.test".to_owned(),
                password: password.to_owned(),
                urls: vec!["https://new.example.test".to_owned()],
                notes: None,
            }),
            ObservedSet::new(),
        )
        .unwrap()
    }

    fn initialized() -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
    ) {
        initialized_with(
            b"active passphrase",
            GenerationZeroRandomness::new(generation_zero_bytes()),
        )
    }

    fn initialized_with(
        passphrase: &[u8],
        randomness: GenerationZeroRandomness,
    ) -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
    ) {
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness,
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(InMemoryObjectStore::new());
        let factory = V1ApplicationRepositoryFactory::from_shared(backend);
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        (locator, local, bootstrap, factory)
    }

    fn pending_publication(active: &ActiveStateV1) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &CatalogV1::empty().encode().unwrap(),
            &ObjectRandomness::new([0xd1; 32], [0xd2; 24], [0xd3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, Vec::new(), catalog_frame)
    }

    fn pending_tombstone_publication(
        active: &ActiveStateV1,
        catalog_item_id: ItemId,
        revision_item_id: ItemId,
        candidate_count: usize,
        causal_parent: Option<RevisionId>,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revision_ids = Vec::new();
        for index in 0..candidate_count {
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                causal_parent,
                ItemState::Tombstone(Tombstone {
                    item_id: revision_item_id,
                    deleted_at_ms: 100 + index as u64,
                }),
            )
            .unwrap();
            let base = 0x40u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revision_ids.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revision_ids.sort_unstable();
        let catalog =
            CatalogV1::new(BTreeMap::from([(catalog_item_id, revision_ids.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xa1; 32], [0xa2; 24], [0xa3; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revision_ids,
        )
    }

    fn pending_live_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
        title: &str,
        password: &str,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut collections = ObservedSet::new();
        collections
            .add(CollectionId::new([0x82; 16]), OperationId::new([0x83; 32]))
            .unwrap();
        let mut tags = ObservedSet::new();
        tags.add("Finance".to_owned(), OperationId::new([0x84; 32]))
            .unwrap();
        let document = ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            100,
            200,
            LwwRegister::new(false, 200, OperationId::new([0x81; 32])),
            collections,
            tags,
            AnyRecord::Login(Login {
                title: title.into(),
                username: "ada@example.test".into(),
                password: password.into(),
                urls: vec!["https://example.test".into()],
                notes: Some("private note".into()),
            }),
            ObservedSet::new(),
        )
        .unwrap();
        let candidate = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [],
            ItemState::Live(Box::new(document)),
        )
        .unwrap();
        let revision_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
            &ObjectRandomness::new([0x91; 32], [0x92; 24], [0x93; 24]),
        )
        .unwrap();
        let revision_id = RevisionId::new(*revision_frame.id().unwrap().as_bytes());
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, vec![revision_id])])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xa4; 32], [0xa5; 24], [0xa6; 24]),
        )
        .unwrap();
        publication_for_catalog(active, vec![revision_frame], catalog_frame)
    }

    fn pending_live_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<(RevisionId, String)>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (title, password)) in
            [("Keep left", "left-secret"), ("Keep right", "right-secret")]
                .into_iter()
                .enumerate()
        {
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(new_login_document(item_id, title, password))),
            )
            .unwrap();
            let base = 0xb0u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            let revision_id = RevisionId::new(*frame.id().unwrap().as_bytes());
            revisions.push((revision_id, title.to_owned()));
            objects.push(frame);
        }
        revisions.sort_unstable_by_key(|(revision_id, _)| *revision_id);
        let catalog = CatalogV1::new(BTreeMap::from([(
            item_id,
            revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect(),
        )]))
        .unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xc0; 32], [0xc1; 24], [0xc2; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_secure_note_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (title, body)) in [
            ("Keep note left", "left-note-secret"),
            ("Keep note right", "right-note-secret"),
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(SECURE_NOTE_V1).unwrap(),
                300,
                300,
                LwwRegister::new(false, 300, OperationId::new([0xd0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::SecureNote(SecureNote {
                    title: title.to_owned(),
                    body: body.to_owned(),
                }),
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xd1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xd7; 32], [0xd8; 24], [0xd9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_card_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (title, number, cvv)) in [
            ("Keep card left", "4111111111111111", "123"),
            ("Keep card right", "5555555555554444", "456"),
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(coding_adventures_vault_records::CARD_V1).unwrap(),
                500,
                500,
                LwwRegister::new(true, 500, OperationId::new([0xe0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::Card(Card {
                    title: title.to_owned(),
                    holder: "Ada Lovelace".to_owned(),
                    number: number.to_owned(),
                    expiry_month: 12,
                    expiry_year: 2030,
                    cvv: cvv.to_owned(),
                    billing_zip: Some("12345".to_owned()),
                }),
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xe1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xe7; 32], [0xe8; 24], [0xe9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_api_key_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (label, token)) in [
            ("Keep key left", "left-token-value"),
            ("Keep key right", "right-token-value"),
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(coding_adventures_vault_records::API_KEY_V1).unwrap(),
                500,
                500,
                LwwRegister::new(true, 500, OperationId::new([0xf0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::ApiKey(ApiKey {
                    label: label.to_owned(),
                    service: "github.com".to_owned(),
                    token: token.to_owned(),
                    scopes: vec!["repo".to_owned()],
                    expires_at: Some(1_900_000_000),
                }),
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xf1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xf7; 32], [0xf8; 24], [0xf9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_database_credential_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (label, password)) in [
            ("Keep database left", "left-password-value"),
            ("Keep database right", "right-password-value"),
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(coding_adventures_vault_records::DATABASE_CREDENTIAL_V1).unwrap(),
                500,
                500,
                LwwRegister::new(true, 500, OperationId::new([0xf0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::DatabaseCredential(DatabaseCredential {
                    label: label.to_owned(),
                    engine: "postgres".to_owned(),
                    host: "db.example".to_owned(),
                    port: 5432,
                    database: Some("orders".to_owned()),
                    username: "service".to_owned(),
                    password: password.to_owned(),
                    lease_id: None,
                    expires_at: None,
                }),
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xf1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xf7; 32], [0xf8; 24], [0xf9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_totp_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        for (index, (label, secret)) in [
            ("Keep seed left", b"left-seed-bytes!".to_vec()),
            ("Keep seed right", b"right-seed-bytes".to_vec()),
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(coding_adventures_vault_records::TOTP_SEED_V1).unwrap(),
                500,
                500,
                LwwRegister::new(true, 500, OperationId::new([0xf0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::TotpSeed(TotpSeed {
                    label: label.to_owned(),
                    issuer: Some("Example".to_owned()),
                    secret,
                    algorithm: "SHA1".to_owned(),
                    digits: 6,
                    period: 30,
                }),
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xf1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xf7; 32], [0xf8; 24], [0xf9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    /// Content type for the opaque-record fixtures.
    ///
    /// It is deliberately not one of the six first-party types, which is the
    /// whole reason `decode_record` would hand these documents back as
    /// `AnyRecord::Opaque`.
    const FIXTURE_OPAQUE_CONTENT_TYPE: &str = "example/future/v1";

    fn pending_opaque_conflict_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
    ) -> (PublicationJournalV1, Vec<RevisionId>) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let mut objects = Vec::new();
        let mut revisions = Vec::new();
        // Each payload is canonical CBOR for a one-entry map: `a1` is a map of
        // one pair, `61 76` is the text key "v", and the value is a text
        // string. Written as bytes rather than through an encoder so that the
        // fixture states the exact wire form the ceremony has to preserve.
        for (index, payload) in [
            vec![0xa1, 0x61, b'v', 0x64, b'l', b'e', b'f', b't'],
            vec![0xa1, 0x61, b'v', 0x65, b'r', b'i', b'g', b'h', b't'],
        ]
        .into_iter()
        .enumerate()
        {
            let document = ItemDocument::new(
                item_id,
                ContentType::new(FIXTURE_OPAQUE_CONTENT_TYPE).unwrap(),
                500,
                500,
                LwwRegister::new(true, 500, OperationId::new([0xf0; 32])),
                ObservedSet::new(),
                ObservedSet::new(),
                AnyRecord::Opaque {
                    content_type: FIXTURE_OPAQUE_CONTENT_TYPE.to_owned(),
                    payload_bytes: payload,
                },
                ObservedSet::new(),
            )
            .unwrap();
            let candidate = ItemCandidate::new(
                RevisionId::new([0; 32]),
                [],
                ItemState::Live(Box::new(document)),
            )
            .unwrap();
            let base = 0xf1u8.wrapping_add(index as u8 * 3);
            let frame = seal_object(
                &keys,
                ObjectKind::ItemRevision,
                &encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap(),
                &ObjectRandomness::new([base; 32], [base + 1; 24], [base + 2; 24]),
            )
            .unwrap();
            revisions.push(RevisionId::new(*frame.id().unwrap().as_bytes()));
            objects.push(frame);
        }
        revisions.sort_unstable();
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, revisions.clone())])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xf7; 32], [0xf8; 24], [0xf9; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, objects, catalog_frame),
            revisions,
        )
    }

    fn pending_dangling_catalog(active: &ActiveStateV1) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog = CatalogV1::new(BTreeMap::from([(
            ItemId::new([0x31; 16]),
            vec![RevisionId::new([0x32; 32])],
        )]))
        .unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xb1; 32], [0xb2; 24], [0xb3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, Vec::new(), catalog_frame)
    }

    fn pending_child_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
        parent_item_id: ItemId,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let parent = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [],
            ItemState::Tombstone(Tombstone {
                item_id: parent_item_id,
                deleted_at_ms: 100,
            }),
        )
        .unwrap();
        let parent_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(parent.causal_parents(), parent.state()).unwrap(),
            &ObjectRandomness::new([0xc1; 32], [0xc2; 24], [0xc3; 24]),
        )
        .unwrap();
        let parent_id = RevisionId::new(*parent_frame.id().unwrap().as_bytes());
        let child = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [parent_id],
            ItemState::Tombstone(Tombstone {
                item_id,
                deleted_at_ms: 101,
            }),
        )
        .unwrap();
        let child_frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &encode_item_revision(child.causal_parents(), child.state()).unwrap(),
            &ObjectRandomness::new([0xd1; 32], [0xd2; 24], [0xd3; 24]),
        )
        .unwrap();
        let child_id = RevisionId::new(*child_frame.id().unwrap().as_bytes());
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, vec![child_id])])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xe1; 32], [0xe2; 24], [0xe3; 24]),
        )
        .unwrap();
        publication_for_catalog(active, vec![parent_frame, child_frame], catalog_frame)
    }

    fn publication_for_catalog(
        active: &ActiveStateV1,
        mut objects: Vec<coding_adventures_vault_pm_format::ObjectFrameV1>,
        catalog_frame: coding_adventures_vault_pm_format::ObjectFrameV1,
    ) -> PublicationJournalV1 {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let signing_seed: [u8; 32] = fixture[168..200].try_into().unwrap();
        let (_, signing_secret) = generate_keypair(&signing_seed);
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let catalog_id = catalog_frame.id().unwrap();
        objects.push(catalog_frame);
        let mut added_objects = objects
            .iter()
            .map(|frame| frame.id().unwrap())
            .collect::<Vec<_>>();
        added_objects.sort_unstable();
        let parents = active.pinned_heads().iter().copied().collect::<Vec<_>>();
        let commit = CommitV1 {
            vault_id: active.vault_id(),
            device_id: active.device_id(),
            device_counter: active.last_device_counter() + 1,
            parents,
            catalog_root: catalog_id,
            added_objects,
            tombstone_root: None,
            wall_time_ms: 20,
            device_certificate: active.device_certificate_id(),
            signature: Signature::new([0; 64]),
        };
        let commit_preimage = commit.signing_preimage().unwrap();
        let commit = commit.with_signature(Signature::new(sign(&commit_preimage, &signing_secret)));
        let commit_frame = seal_object(
            &keys,
            ObjectKind::Commit,
            &encode_signed_commit(&commit).unwrap(),
            &ObjectRandomness::new([0xe1; 32], [0xe2; 24], [0xe3; 24]),
        )
        .unwrap();
        let commit_id = commit_frame.id().unwrap();
        let announcement = AnnouncementV1 {
            vault_id: active.vault_id(),
            device_id: active.device_id(),
            device_counter: commit.device_counter,
            commit_id,
            device_certificate: active.device_certificate_id(),
            signature: Signature::new([0; 64]),
        };
        let announcement_preimage = announcement.signing_preimage().unwrap();
        let announcement = announcement.with_signature(Signature::new(sign(
            &announcement_preimage,
            &signing_secret,
        )));
        PublicationJournalV1::new(
            objects,
            commit_frame,
            announcement.encode().unwrap(),
            active.pinned_heads().clone(),
            PinnedHeads::new([commit_id]).unwrap(),
            commit.device_counter,
            catalog_id,
        )
        .unwrap()
    }

    fn install_pending(local: &MemoryLocalStateStore) -> PublicationJournalV1 {
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let publication = pending_publication(&active);
        let pending = LocalVaultStateV1::pending_publication(active, publication.clone()).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        publication
    }

    #[test]
    fn active_vault_reopens_from_only_durable_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(session.local_pins(), session.open_report().heads());
        assert_eq!(session.vault_id(), session.active.vault_id());
        assert_eq!(session.device_id(), session.active.device_id());
        assert_eq!(session.open_report().announcement_count(), 1);
        assert_eq!(session.open_report().commit_count(), 1);
        assert!(!session.open_report().fresh_device_unanchored());
        assert_eq!(session.item_count(), 0);
        assert_eq!(session.candidate_count(), 0);
        assert_eq!(session.conflicted_item_count(), 0);
        assert_eq!(session.search_item_count(), 0);
        assert_eq!(
            format!("{session:?}"),
            "UnlockedVaultV1 { local_pin_count: 1, verified_head_count: 1, item_count: 0, candidate_count: 0, conflicted_item_count: 0, search_item_count: 0, .. }"
        );
    }

    #[test]
    fn audit_verify_reopens_complete_ancestry_and_reports_only_counts() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let initial = session.audit_verify().unwrap();
        assert!(initial.integrity_verified());
        assert_eq!(initial.announcement_count(), 1);
        assert_eq!(initial.commit_count(), 1);
        assert_eq!(initial.catalog_count(), 1);
        assert_eq!(initial.revision_count(), 0);
        assert_eq!(initial.item_count(), 0);
        assert_eq!(initial.audit_event_count(), 0);

        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            699,
            None,
            None,
            [0xa7; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let epoch = session.audit_verify().unwrap();
        assert_eq!(epoch.commit_count(), 2);
        assert_eq!(epoch.catalog_count(), 1);
        assert_eq!(epoch.audit_event_count(), 1);

        publish_audit_only_event_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            AuditActionV1::ItemList,
            AuditOutcomeV1::Succeeded,
            None,
            None,
            699,
            None,
            None,
            [0xaa; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let accessed = session.audit_verify().unwrap();
        assert_eq!(accessed.commit_count(), 3);
        assert_eq!(accessed.catalog_count(), 1);
        assert_eq!(accessed.audit_event_count(), 2);

        let randomness = add_item_randomness(0xa8);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "Audited login", "audit-secret"),
                700,
                randomness,
                &local,
            )
            .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.announcement_count(), 4);
        assert_eq!(report.commit_count(), 4);
        assert_eq!(report.catalog_count(), 2);
        assert_eq!(report.revision_count(), 1);
        assert_eq!(report.item_count(), 1);
        assert_eq!(report.audit_event_count(), 3);
        assert_eq!(
            format!("{report:?}"),
            "AuditVerificationV1 { integrity_verified: true, announcement_count: 4, commit_count: 4, catalog_count: 2, revision_count: 1, item_count: 1, audit_event_count: 3 }"
        );
        assert!(!format!("{report:?}").contains("Audited login"));
        assert!(!format!("{report:?}").contains("audit-secret"));

        let expected_revision = reopened.current_catalog.items[&item_id][0].revision_id();
        reopened
            .replace_item(
                expected_revision,
                login_document_with_times(
                    item_id,
                    "Audited login updated",
                    "audit-secret-updated",
                    300,
                    701,
                ),
                702,
                replace_item_randomness(0xa9),
                &local,
            )
            .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.announcement_count(), 5);
        assert_eq!(report.commit_count(), 5);
        assert_eq!(report.catalog_count(), 3);
        assert_eq!(report.revision_count(), 2);
        assert_eq!(report.item_count(), 1);
        assert_eq!(report.audit_event_count(), 4);
    }

    #[test]
    fn audit_only_publication_replays_after_ambiguous_provider_failure() {
        let passphrase = b"active passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let catalog_root = session.active.catalog_root();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert!(matches!(
            session.activate_audit_epoch(703, audited_access_randomness(0xab), &local),
            Err(ApplicationError::StorageUnavailable)
        ));
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::PendingPublication {
            active,
            publication,
        } = LocalVaultStateV1::decode(&exact_pending).unwrap()
        else {
            panic!("audit-only failure must retain the exact pending journal")
        };
        assert_eq!(active.catalog_root(), catalog_root);
        assert_eq!(publication.catalog_root(), catalog_root);
        assert_eq!(publication.objects().len(), 1);
        assert_eq!(
            publication.audit_event_head(),
            publication.objects()[0].id().ok()
        );
        let recovered = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(recovered.catalog_root(), catalog_root);
        assert_eq!(recovered.last_device_counter(), 2);
        assert!(recovered.audit_event_head().is_some());
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 2);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 1);
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    // -----------------------------------------------------------------------
    // VLT-PM42 — the recovering unlock
    //
    // `recover_pending_publication` was correct and unreachable. These tests
    // cover the lifecycle boundary that finally reaches it, and they build
    // their wedged vault the way a crash builds one — by letting a real
    // publication start and then taking the provider away — rather than by
    // hand-assembling journal bytes.
    // -----------------------------------------------------------------------

    /// One vault whose durable owner state is an exact `PendingPublication`,
    /// plus those exact pending bytes for later comparison.
    #[allow(clippy::type_complexity)]
    fn wedged_by_an_interrupted_publication(
        passphrase: &[u8],
    ) -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<FaultInjectingObjectStore<InMemoryObjectStore>>,
        Vec<u8>,
    ) {
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();

        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        // The commit lands and *then* the provider stops answering: exactly the
        // ambiguity a `SIGKILL` between the write-ahead journal and the
        // owner-state advance produces.
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();
        assert!(matches!(
            session.activate_audit_epoch(703, audited_access_randomness(0xab), &local),
            Err(ApplicationError::StorageUnavailable)
        ));

        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&exact_pending).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));
        (locator, local, bootstrap, factory, exact_pending)
    }

    #[test]
    fn a_recovering_unlock_leaves_an_active_vault_alone() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let mut access = crate::VaultAccessV1::locked(locator);

        let outcome = access
            .unlock_recovering_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

        assert_eq!(outcome, crate::UnlockRecoveryV1::AlreadyActive);
        assert!(access.is_unlocked());
        // Nothing was published, so the durable bytes are the ones we started
        // with, byte for byte.
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
    }

    #[test]
    fn a_recovering_unlock_finishes_an_interrupted_publication_and_opens() {
        let passphrase = b"active passphrase";
        let (locator, local, bootstrap, factory, exact_pending) =
            wedged_by_an_interrupted_publication(passphrase);
        let LocalVaultStateV1::PendingPublication {
            active: interrupted,
            publication,
        } = LocalVaultStateV1::decode(&exact_pending).unwrap()
        else {
            panic!("fixture must be pending")
        };

        let mut access = crate::VaultAccessV1::locked(locator);
        let outcome = access
            .unlock_recovering_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

        assert_eq!(
            outcome,
            crate::UnlockRecoveryV1::RecoveredPendingPublication
        );
        // The durable end state is exactly what the journal intended, which is
        // the same state `recover_pending_publication` produces on its own.
        let intended = interrupted.after_publication(&publication).unwrap();
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(
                LocalVaultStateV1::Active(intended)
                    .encode()
                    .unwrap()
                    .as_slice()
            )
        );
        // And the session is a real one: the recovered audit epoch is present
        // and the whole chain verifies.
        let session = access.into_unlocked().unwrap();
        assert!(session.audit_enabled());
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 2);
        assert_eq!(report.audit_event_count(), 1);
    }

    #[test]
    fn a_recovering_unlock_is_idempotent() {
        let passphrase = b"active passphrase";
        let (locator, local, bootstrap, factory, _) =
            wedged_by_an_interrupted_publication(passphrase);

        let mut access = crate::VaultAccessV1::locked(locator);
        access
            .unlock_recovering_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        let recovered = local.0.lock().unwrap().clone().unwrap();

        // A second process finds an ordinary vault and publishes nothing more.
        let mut again = crate::VaultAccessV1::locked(locator);
        let outcome = again
            .unlock_recovering_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(outcome, crate::UnlockRecoveryV1::AlreadyActive);
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(recovered.as_slice())
        );
    }

    #[test]
    fn a_recovering_unlock_refuses_the_wrong_passphrase_and_keeps_the_journal() {
        let (locator, local, bootstrap, factory, exact_pending) =
            wedged_by_an_interrupted_publication(b"active passphrase");

        let mut access = crate::VaultAccessV1::locked(locator);
        assert!(matches!(
            access.unlock_recovering_pending_publication(
                Zeroizing::new(b"wrong passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::AuthenticationFailed)
        ));

        // Fails closed: still locked, and the exact journal is untouched so a
        // later attempt with the right secret still repairs the vault.
        assert!(access.is_locked());
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );
        let outcome = access
            .unlock_recovering_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(
            outcome,
            crate::UnlockRecoveryV1::RecoveredPendingPublication
        );
    }

    #[test]
    fn a_recovering_unlock_refuses_a_prepared_initialization() {
        // A `PreparedInit` journal is `init`'s to finish, not an unlock's: it
        // has no signed publication to replay and no `Active` state to open.
        let passphrase = b"prepared passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let factory =
            V1ApplicationRepositoryFactory::from_shared(Arc::new(InMemoryObjectStore::new()));
        let exact_prepared = prepared.owner_state().encode().unwrap();
        *local.0.lock().unwrap() = Some(exact_prepared.clone());

        let mut access = crate::VaultAccessV1::locked(locator);
        assert!(matches!(
            access.unlock_recovering_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::InvalidInput)
        ));
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_prepared.as_slice())
        );
    }

    #[test]
    fn a_plain_unlock_still_refuses_a_pending_publication() {
        // The strict door stays strict. A host that wants "open an `Active`
        // vault and refuse anything else" still has exactly that.
        let (locator, local, bootstrap, factory, exact_pending) =
            wedged_by_an_interrupted_publication(b"active passphrase");

        let mut access = crate::VaultAccessV1::locked(locator);
        assert!(matches!(
            access.unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::InvalidInput)
        ));
        assert!(access.is_locked());
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );
    }

    #[test]
    fn a_recovering_unlock_refuses_an_already_unlocked_boundary() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut access = crate::VaultAccessV1::locked(locator);
        access
            .unlock_recovering_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert!(matches!(
            access.unlock_recovering_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::InvalidInput)
        ));
    }

    #[test]
    fn audited_list_refuses_pre_audit_sessions_without_changing_owner_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert!(matches!(
            session.audited_list_items(704, audited_access_randomness(0xac), &local),
            Err(ApplicationError::InvalidInput)
        ));
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
    }

    #[test]
    fn production_audit_epoch_activation_is_durable_single_use_and_verifiable() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let active = session
            .activate_audit_epoch(704, audited_access_randomness(0xac), &local)
            .unwrap();
        assert!(active.audit_event_head().is_some());
        let exact_active = local.0.lock().unwrap().clone().unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::AuditEpochStart,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 2);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 1);
        assert!(matches!(
            session.activate_audit_epoch(705, audited_access_randomness(0xad), &local),
            Err(ApplicationError::InvalidInput)
        ));
        assert_eq!(*local.0.lock().unwrap(), Some(exact_active));
    }

    #[test]
    fn audit_history_logs_itself_before_bounded_list_and_trace_lookup() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        session
            .activate_audit_epoch(704, audited_access_randomness(0xac), &local)
            .unwrap();

        let exact_epoch = local.0.lock().unwrap().clone().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert!(matches!(
            session.audited_audit_history(0, 705, audited_access_randomness(0xad), &local),
            Err(ApplicationError::BoundExceeded)
        ));
        assert_eq!(*local.0.lock().unwrap(), Some(exact_epoch.clone()));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert!(matches!(
            session.audited_audit_history(
                crate::MAX_AUDIT_HISTORY_LIMIT + 1,
                705,
                audited_access_randomness(0xad),
                &local,
            ),
            Err(ApplicationError::BoundExceeded)
        ));
        assert_eq!(*local.0.lock().unwrap(), Some(exact_epoch));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let listed = session
            .audited_audit_history(1, 706, audited_access_randomness(0xae), &local)
            .unwrap();
        assert_eq!(listed.active_state().last_device_counter(), 3);
        let views = listed.into_operation().unwrap();
        assert_eq!(views.len(), 1);
        let own_access = &views[0];
        assert_eq!(own_access.trace_id(), audited_access_trace(0xae));
        assert_eq!(own_access.device_counter(), 3);
        assert_eq!(own_access.action(), AuditActionV1::AuditRead);
        assert_eq!(own_access.action().label(), "audit_read");
        assert_eq!(own_access.outcome(), AuditOutcomeV1::Succeeded);
        assert_eq!(own_access.timestamp_ms(), 706);
        assert_eq!(own_access.item_id(), None);
        assert_eq!(own_access.selected_revision(), None);
        assert_eq!(own_access.result_revision(), None);

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let shown = session
            .audited_audit_event(
                audited_access_trace(0xac),
                707,
                audited_access_randomness(0xaf),
                &local,
            )
            .unwrap();
        assert_eq!(shown.active_state().last_device_counter(), 4);
        let epoch = shown.into_operation().unwrap().unwrap();
        assert_eq!(epoch.trace_id(), audited_access_trace(0xac));
        assert_eq!(epoch.device_counter(), 2);
        assert_eq!(epoch.action(), AuditActionV1::AuditEpochStart);
        assert_eq!(epoch.outcome(), AuditOutcomeV1::Succeeded);
        assert_eq!(epoch.timestamp_ms(), 704);

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let missing = session
            .audited_audit_event(
                OperationId::new([0xff; 32]),
                708,
                audited_access_randomness(0xb0),
                &local,
            )
            .unwrap();
        assert!(missing.operation_succeeded());
        assert!(missing.into_operation().unwrap().is_none());

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 5);
        assert_eq!(report.audit_event_count(), 4);
    }

    #[test]
    fn audited_list_releases_redacted_results_only_with_durable_next_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            705,
            None,
            None,
            [0xad; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let catalog_root = session.active.catalog_root();
        let prior_counter = session.active.last_device_counter();

        let audited = session
            .audited_list_items(706, audited_access_randomness(0xae), &local)
            .unwrap();
        assert!(audited.operation_succeeded());
        assert_eq!(audited.active_state().catalog_root(), catalog_root);
        assert_eq!(
            audited.active_state().last_device_counter(),
            prior_counter + 1
        );
        assert_eq!(
            format!("{audited:?}"),
            "AuditedAccessResultV1 { operation_succeeded: true, .. }"
        );
        let (_, operation) = audited.into_parts();
        assert_eq!(operation.unwrap(), Vec::new());

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 3);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 2);
        let event_head = reopened.active.audit_event_head().unwrap();
        let object = reopened._repository.read_object(event_head).unwrap();
        let plaintext =
            open_object(&reopened._keys, ObjectKind::AuditEvent, object.frame()).unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        assert_eq!(event.event().action(), AuditActionV1::ItemList);
        assert_eq!(event.event().outcome(), AuditOutcomeV1::Succeeded);
    }

    #[test]
    fn audited_list_records_post_authentication_conflict_before_releasing_failure() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0xaf; 16]);
        let (publication, _) = pending_live_conflict_publication(&active, item_id);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            707,
            None,
            None,
            [0xb0; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let catalog_root = session.active.catalog_root();

        let audited = session
            .audited_list_items(708, audited_access_randomness(0xb1), &local)
            .unwrap();
        assert!(!audited.operation_succeeded());
        assert_eq!(audited.active_state().catalog_root(), catalog_root);
        let (_, operation) = audited.into_parts();
        assert_eq!(operation, Err(ApplicationError::ConflictRequired));

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.catalog_count(), 2);
        assert_eq!(report.audit_event_count(), 2);
        let event_head = reopened.active.audit_event_head().unwrap();
        let object = reopened._repository.read_object(event_head).unwrap();
        let plaintext =
            open_object(&reopened._keys, ObjectKind::AuditEvent, object.frame()).unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        assert_eq!(event.event().action(), AuditActionV1::ItemList);
        assert_eq!(event.event().outcome(), AuditOutcomeV1::Failed);
    }

    #[test]
    fn audited_list_and_history_withhold_results_during_ambiguous_provider_failure() {
        let passphrase = b"active passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            709,
            None,
            None,
            [0xb2; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let catalog_root = session.active.catalog_root();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert!(matches!(
            session.audited_list_items(710, audited_access_randomness(0xb3), &local),
            Err(ApplicationError::StorageUnavailable)
        ));
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::PendingPublication {
            active,
            publication,
        } = LocalVaultStateV1::decode(&exact_pending).unwrap()
        else {
            panic!("audited access failure must retain the exact pending journal")
        };
        assert_eq!(active.catalog_root(), catalog_root);
        assert_eq!(publication.catalog_root(), catalog_root);
        assert_eq!(publication.objects().len(), 1);
        assert_eq!(
            publication.audit_event_head(),
            publication.objects()[0].id().ok()
        );

        let recovered = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(recovered.catalog_root(), catalog_root);
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 3);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 2);
        assert_eq!(backend.pending_faults().unwrap(), 0);
        drop(reopened);

        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();
        assert!(matches!(
            session.audited_audit_history(100, 711, audited_access_randomness(0xb4), &local),
            Err(ApplicationError::StorageUnavailable)
        ));
        assert!(matches!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));

        recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let report = reopened.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 4);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 3);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::AuditRead,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn audited_redacted_reads_form_one_exact_success_and_failure_chain() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xc1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Audited redacted portal", "hidden-secret"),
            720,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let selected_revision = session.current_catalog.items[&item_id][0].revision_id();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            721,
            None,
            None,
            [0xc2; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let (_, shown) = session
            .audited_get_item(item_id, 722, audited_access_randomness(0xc3), &local)
            .unwrap()
            .into_parts();
        let shown = shown.unwrap();
        assert_eq!(shown.item_id, item_id);
        assert!(matches!(
            &shown.record,
            RedactedRecordView::Login { title, .. } if title == "Audited redacted portal"
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(selected_revision),
            )
        );

        let (_, searched) = session
            .audited_search_items(
                Zeroizing::new("redacted portal".to_owned()),
                None,
                10,
                723,
                audited_access_randomness(0xc4),
                &local,
            )
            .unwrap()
            .into_parts();
        assert_eq!(searched.unwrap().len(), 1);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemSearch,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );

        let (_, history) = session
            .audited_item_history(
                item_id,
                DEFAULT_ITEM_HISTORY_LIMIT,
                724,
                audited_access_randomness(0xc5),
                &local,
            )
            .unwrap()
            .into_parts();
        assert_eq!(history.unwrap().len(), 1);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemHistoryRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );

        let (_, candidates) = session
            .audited_conflict_candidates(item_id, 725, audited_access_randomness(0xc6), &local)
            .unwrap()
            .into_parts();
        assert_eq!(candidates, Err(ApplicationError::ConflictRequired));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemHistoryRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );

        let missing = ItemId::new([0xc7; 16]);
        let (_, missing_result) = session
            .audited_get_item(missing, 726, audited_access_randomness(0xc8), &local)
            .unwrap()
            .into_parts();
        assert_eq!(missing_result, Err(ApplicationError::NotFound));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(missing),
                None,
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 8);
        assert_eq!(report.catalog_count(), 2);
        assert_eq!(report.audit_event_count(), 6);
    }

    #[test]
    fn audited_verification_diagnostics_and_export_form_one_exact_chain() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            730,
            None,
            None,
            [0xd1; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let (_, verification) = session
            .audited_verify(731, audited_access_randomness(0xd2), &local)
            .unwrap()
            .into_parts();
        let verification = verification.unwrap();
        assert_eq!(verification.commit_count(), 2);
        assert_eq!(verification.catalog_count(), 1);
        assert_eq!(verification.audit_event_count(), 1);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::VaultVerify,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );

        let (_, diagnosis) = session
            .audited_doctor(&local, &bootstrap, 732, audited_access_randomness(0xd3))
            .unwrap()
            .into_parts();
        assert_eq!(
            diagnosis.unwrap().state(),
            crate::VaultDoctorStateV1::Healthy
        );
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::VaultDiagnose,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );

        let policy = crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        let (_, exported) = session
            .audited_export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                crate::PortableExportRandomnessV1::new([0xd4; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
                733,
                audited_access_randomness(0xd5),
                &local,
            )
            .unwrap()
            .into_parts();
        assert!(!exported.unwrap().as_bytes().is_empty());
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::PortableExport,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );

        let (_, rejected) = session
            .audited_export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(Vec::new()),
                policy,
                crate::PortableExportRandomnessV1::new([0xd6; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
                734,
                audited_access_randomness(0xd7),
                &local,
            )
            .unwrap()
            .into_parts();
        assert!(matches!(rejected, Err(ApplicationError::InvalidInput)));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::PortableExport,
                AuditOutcomeV1::Failed,
                None,
                None,
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 6);
        assert_eq!(report.catalog_count(), 1);
        assert_eq!(report.audit_event_count(), 5);
    }

    #[test]
    fn audited_revision_capabilities_and_secret_disclosure_form_one_exact_chain() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xe1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Audited secret portal", "held-secret"),
            740,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_revision = session.current_catalog.items[&item_id][0].revision_id();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            741,
            None,
            None,
            [0xe2; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let (_, capability) = session
            .audited_current_item_revision(item_id, 742, audited_access_randomness(0xe3), &local)
            .unwrap()
            .into_parts();
        assert_eq!(capability.unwrap(), exact_revision);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let (_, document) = session
            .audited_reveal_item_revision(
                item_id,
                exact_revision,
                743,
                audited_access_randomness(0xe4),
                &local,
            )
            .unwrap()
            .into_parts();
        let document = document.unwrap();
        let AnyRecord::Login(login) = document.payload() else {
            panic!("fixture must reveal a login")
        };
        assert_eq!(login.password, "held-secret");
        drop(document);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let (_, denied) = session
            .audited_reveal_item_revision_field(
                item_id,
                RevisionId::new([0xea; 32]),
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: false },
                744,
                audited_access_randomness(0xe5),
                &local,
            )
            .unwrap()
            .into_parts();
        assert!(matches!(denied, Err(ApplicationError::InvalidInput)));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Denied,
                Some(item_id),
                None,
            )
        );

        let (_, revealed) = session
            .audited_reveal_item_revision_field(
                item_id,
                exact_revision,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::Clipboard,
                745,
                audited_access_randomness(0xe6),
                &local,
            )
            .unwrap()
            .into_parts();
        let revealed = revealed.unwrap();
        assert_eq!(revealed.as_bytes(), b"held-secret");
        drop(revealed);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let (_, wrong_field) = session
            .audited_reveal_item_revision_field(
                item_id,
                exact_revision,
                SecretFieldV1::CardCvv,
                SecretDisclosureIntentV1::Clipboard,
                746,
                audited_access_randomness(0xe7),
                &local,
            )
            .unwrap()
            .into_parts();
        assert!(matches!(wrong_field, Err(ApplicationError::InvalidInput)));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let missing = ItemId::new([0xe8; 16]);
        let (_, missing_capability) = session
            .audited_current_item_revision(missing, 747, audited_access_randomness(0xe9), &local)
            .unwrap()
            .into_parts();
        assert_eq!(missing_capability, Err(ApplicationError::NotFound));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(missing),
                None,
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 9);
        assert_eq!(report.catalog_count(), 2);
        assert_eq!(report.audit_event_count(), 7);
    }

    #[test]
    fn audited_current_secret_disclosure_keeps_revision_capability_inside_application() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x61);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Current disclosure", "current-secret"),
            750,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_revision = session.current_catalog.items[&item_id][0].revision_id();
        activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            751,
            None,
            None,
            [0x62; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        drop(session);

        let denied = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_reveal_current_item_field(
            item_id,
            SecretFieldV1::LoginPassword,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: false },
            752,
            audited_access_randomness(0x63),
            &local,
        )
        .unwrap();
        assert!(matches!(
            denied.into_operation(),
            Err(ApplicationError::InvalidInput)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Denied,
                Some(item_id),
                None,
            )
        );

        let revealed = session
            .audited_reveal_current_item_field(
                item_id,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                753,
                audited_access_randomness(0x64),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();
        assert_eq!(revealed.as_bytes(), b"current-secret");
        drop(revealed);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let wrong_field = session
            .audited_reveal_current_item_field(
                item_id,
                SecretFieldV1::CardCvv,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                754,
                audited_access_randomness(0x65),
                &local,
            )
            .unwrap();
        assert!(matches!(
            wrong_field.into_operation(),
            Err(ApplicationError::InvalidInput)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(exact_revision),
            )
        );

        let missing = ItemId::new([0x66; 16]);
        let missing_result = session
            .audited_reveal_current_item_field(
                missing,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                755,
                audited_access_randomness(0x67),
                &local,
            )
            .unwrap();
        assert!(matches!(
            missing_result.into_operation(),
            Err(ApplicationError::NotFound)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(missing),
                None,
            )
        );

        session
            .delete_current_item(item_id, 756, 756, delete_item_randomness(0x68), &local)
            .unwrap();
        let tombstoned = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_reveal_current_item_field(
            item_id,
            SecretFieldV1::LoginPassword,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
            757,
            audited_access_randomness(0x69),
            &local,
        )
        .unwrap();
        assert!(matches!(
            tombstoned.into_operation(),
            Err(ApplicationError::NotFound)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_current_secret_disclosure_rejects_conflict_without_selecting_candidate() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x6a; 16]);
        let (publication, _) = pending_live_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(758, audited_access_randomness(0x6b), &local)
        .unwrap();

        let conflicted = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_reveal_current_item_field(
            item_id,
            SecretFieldV1::LoginPassword,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
            759,
            audited_access_randomness(0x6c),
            &local,
        )
        .unwrap();
        assert!(matches!(
            conflicted.into_operation(),
            Err(ApplicationError::ConflictRequired)
        ));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_conflict_candidate_disclosure_requires_exact_current_membership() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x6d; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(760, audited_access_randomness(0x6e), &local)
        .unwrap();

        let selected_revision = revisions[0].0;
        let expected_secret = match revisions[0].1.as_str() {
            "Keep left" => b"left-secret".as_slice(),
            "Keep right" => b"right-secret".as_slice(),
            _ => panic!("unexpected fixture title"),
        };
        let denied = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_reveal_conflict_candidate_field(
            item_id,
            RevisionId::new([0xff; 32]),
            SecretFieldV1::LoginPassword,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: false },
            761,
            audited_access_randomness(0x6f),
            &local,
        )
        .unwrap();
        assert!(matches!(
            denied.into_operation(),
            Err(ApplicationError::InvalidInput)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Denied,
                Some(item_id),
                None,
            )
        );

        let revealed = session
            .audited_reveal_conflict_candidate_field(
                item_id,
                selected_revision,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                762,
                audited_access_randomness(0x70),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();
        assert_eq!(revealed.as_bytes(), expected_secret);
        drop(revealed);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(selected_revision),
            )
        );

        let wrong_field = session
            .audited_reveal_conflict_candidate_field(
                item_id,
                selected_revision,
                SecretFieldV1::CardCvv,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                763,
                audited_access_randomness(0x71),
                &local,
            )
            .unwrap();
        assert!(matches!(
            wrong_field.into_operation(),
            Err(ApplicationError::InvalidInput)
        ));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(selected_revision),
            )
        );

        let noncandidate = session
            .audited_reveal_conflict_candidate_field(
                item_id,
                RevisionId::new([0xfe; 32]),
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                764,
                audited_access_randomness(0x72),
                &local,
            )
            .unwrap();
        assert!(matches!(
            noncandidate.into_operation(),
            Err(ApplicationError::NotFound)
        ));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let wrong_item = ItemId::new([0xfd; 16]);
        let wrong_item_result = reopened
            .audited_reveal_conflict_candidate_field(
                wrong_item,
                selected_revision,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
                765,
                audited_access_randomness(0x73),
                &local,
            )
            .unwrap();
        assert!(matches!(
            wrong_item_result.into_operation(),
            Err(ApplicationError::NotFound)
        ));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(wrong_item),
                None,
            )
        );
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            reopened.current_catalog.items[&item_id]
                .iter()
                .map(ItemCandidate::revision_id)
                .collect::<Vec<_>>(),
            revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn audited_conflict_candidate_disclosure_binds_a_current_tombstone_failure() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x73; 16]);
        let (publication, revisions) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(766, audited_access_randomness(0x74), &local)
        .unwrap();

        let selected_revision = revisions[0];
        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_reveal_conflict_candidate_field(
            item_id,
            selected_revision,
            SecretFieldV1::LoginPassword,
            SecretDisclosureIntentV1::InteractiveReveal { confirmed: true },
            767,
            audited_access_randomness(0x75),
            &local,
        )
        .unwrap();
        assert!(matches!(
            result.into_operation(),
            Err(ApplicationError::InvalidInput)
        ));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemRead,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(selected_revision),
            )
        );
        assert_eq!(reopened.conflicted_item_count(), 1);
    }

    #[test]
    fn audit_verify_rejects_wrong_event_basis_and_signer() {
        for (basis_override, signing_seed_override, randomness) in [
            (
                Some(vec![ObjectId::new([0xfe; 32])]),
                None,
                [0xb1; AUDIT_ONLY_TEST_RANDOM_BYTES],
            ),
            (None, Some([0xfd; 32]), [0xb2; AUDIT_ONLY_TEST_RANDOM_BYTES]),
        ] {
            let (locator, local, bootstrap, factory) = initialized();
            let session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            activate_audit_epoch_for_test(
                &session.active,
                &session._keys,
                &session._local_secret,
                session._repository.as_ref(),
                800,
                basis_override,
                signing_seed_override,
                randomness,
                &local,
            )
            .unwrap();
            drop(session);
            let reopened = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(
                reopened.audit_verify(),
                Err(ApplicationError::IntegrityFailure)
            );
        }
    }

    #[test]
    fn audit_verify_rejects_a_skipped_durable_event_head() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let epoch = activate_audit_epoch_for_test(
            &session.active,
            &session._keys,
            &session._local_secret,
            session._repository.as_ref(),
            810,
            None,
            None,
            [0xb3; AUDIT_ONLY_TEST_RANDOM_BYTES],
            &local,
        )
        .unwrap();
        let epoch_head = epoch.audit_event_head().unwrap();
        drop(session);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0xb4);
        let item_id = randomness.item_id();
        let latest = session
            .add_item(
                new_login_document(item_id, "Linked audit", "linked-secret"),
                811,
                randomness,
                &local,
            )
            .unwrap();
        let exact_latest = LocalVaultStateV1::Active(latest.clone()).encode().unwrap();
        let skipped = latest.with_audit_event_head(epoch_head).unwrap();
        let exact_skipped = LocalVaultStateV1::Active(skipped).encode().unwrap();
        local
            .compare_exchange(locator, Some(&exact_latest), &exact_skipped)
            .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            reopened.audit_verify(),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_export_encrypts_every_current_candidate_under_a_separate_passphrase() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0xaa);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "Portable portal", "portable-export-secret"),
                800,
                randomness,
                &local,
            )
            .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        let policy = crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        let export_randomness = [0x5b; crate::PORTABLE_EXPORT_RANDOM_BYTES];
        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                crate::PortableExportRandomnessV1::new(export_randomness),
            )
            .unwrap();
        let repeated = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                crate::PortableExportRandomnessV1::new(export_randomness),
            )
            .unwrap();
        assert_eq!(artifact.as_bytes(), repeated.as_bytes());
        assert_eq!(
            coding_adventures_sha256::sha256(artifact.as_bytes()),
            [
                0xb8, 0xb7, 0x05, 0x87, 0xea, 0x11, 0x3b, 0x11, 0xea, 0x23, 0xa7, 0x7a, 0x9b, 0x36,
                0xe6, 0xc4, 0x0e, 0x51, 0x04, 0x86, 0x99, 0x60, 0xb9, 0x38, 0x98, 0x93, 0xaf, 0x5e,
                0x96, 0x53, 0xa1, 0x3a,
            ]
        );
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 1);
        assert_eq!(opened.candidate_count(), 1);
        assert_eq!(
            format!("{opened:?}"),
            "OpenedPortableSnapshotV1(<redacted>)"
        );
        for plaintext in [b"Portable portal".as_slice(), b"portable-export-secret"] {
            assert!(!artifact
                .as_bytes()
                .windows(plaintext.len())
                .any(|window| window == plaintext));
        }
        assert!(crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"wrong export passphrase".to_vec()),
        )
        .is_none());
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(b"wrong export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        let mut tampered = artifact.as_bytes().to_vec();
        let last = tampered.last_mut().unwrap();
        *last ^= 1;
        assert!(crate::export::decrypt_portable_for_test(
            &tampered,
            Zeroizing::new(b"separate export passphrase".to_vec()),
        )
        .is_none());
        assert_eq!(
            crate::open_portable_with_passphrase(
                &tampered,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );

        let plaintext = crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
        )
        .unwrap();
        let CborValue::Map(mut snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!("portable snapshot must be a canonical map")
        };
        assert_eq!(take_cbor_field(&mut snapshot, 1), CborValue::Unsigned(1));
        let CborValue::Bytes(exported_bootstrap) = take_cbor_field(&mut snapshot, 2) else {
            panic!()
        };
        assert_eq!(exported_bootstrap, exact_bootstrap);
        let mut entries = take_cbor_field(&mut snapshot, 3);
        let encoded_entries = encode_cbor(&entries);
        let CborValue::Unsigned(candidate_count) = take_cbor_field(&mut snapshot, 4) else {
            panic!()
        };
        assert_eq!(candidate_count, 1);
        let CborValue::Bytes(exported_hash) = take_cbor_field(&mut snapshot, 5) else {
            panic!()
        };
        assert_eq!(
            exported_hash,
            crate::export::snapshot_hash(&exact_bootstrap, &encoded_entries)
                .unwrap()
                .to_vec()
        );
        assert!(snapshot.is_empty());

        let CborValue::Array(exported_candidates) = &mut entries else {
            panic!()
        };
        assert_eq!(exported_candidates.len(), 1);
        let CborValue::Map(mut entry) = exported_candidates.remove(0) else {
            panic!()
        };
        let CborValue::Bytes(exported_item_id) = take_cbor_field(&mut entry, 1) else {
            panic!()
        };
        assert_eq!(exported_item_id, item_id.as_bytes());
        let CborValue::Bytes(exported_revision_id) = take_cbor_field(&mut entry, 2) else {
            panic!()
        };
        let revision_id = RevisionId::new(exported_revision_id.try_into().unwrap());
        let CborValue::Bytes(mut encoded_revision) = take_cbor_field(&mut entry, 3) else {
            panic!()
        };
        assert!(entry.is_empty());
        let candidate = crate::decode_item_revision(revision_id, &encoded_revision).unwrap();
        let ItemState::Live(document) = candidate.state() else {
            panic!()
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!()
        };
        assert_eq!(login.title, "Portable portal");
        assert_eq!(login.password, "portable-export-secret");
        encoded_revision.zeroize();
        crate::export::zeroize_cbor(&mut entries);

        let CborValue::Map(mut invalid_count) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        replace_cbor_field(&mut invalid_count, 4, CborValue::Unsigned(2));
        let invalid_count =
            authenticate_portable_snapshot(invalid_count, b"separate export passphrase", 0x5c);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_count,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut invalid_hash) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        replace_cbor_field(&mut invalid_hash, 5, CborValue::Bytes(vec![0; 32]));
        let invalid_hash =
            authenticate_portable_snapshot(invalid_hash, b"separate export passphrase", 0x5d);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_hash,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut invalid_bootstrap) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Bytes(bootstrap_bytes) = invalid_bootstrap
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(2))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        *bootstrap_bytes.last_mut().unwrap() ^= 1;
        refresh_portable_snapshot_hash(&mut invalid_bootstrap);
        let invalid_bootstrap =
            authenticate_portable_snapshot(invalid_bootstrap, b"separate export passphrase", 0x5e);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &invalid_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let CborValue::Map(mut mismatched_item) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Array(candidate_entries) = mismatched_item
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        let CborValue::Map(candidate_fields) = &mut candidate_entries[0] else {
            panic!()
        };
        replace_cbor_field(candidate_fields, 1, CborValue::Bytes(vec![0xfe; 16]));
        refresh_portable_snapshot_hash(&mut mismatched_item);
        let mismatched_item =
            authenticate_portable_snapshot(mismatched_item, b"separate export passphrase", 0x5f);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &mismatched_item,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            format!("{artifact:?}"),
            "PortableExportArtifactV1(<encrypted>)"
        );
    }

    #[test]
    fn portable_export_rejects_credential_and_bootstrap_misuse() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        let policy = crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap();
        let randomness =
            || crate::PortableExportRandomnessV1::new([0x6c; crate::PORTABLE_EXPORT_RANDOM_BYTES]);
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &exact_bootstrap,
                    Zeroizing::new(Vec::new()),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &exact_bootstrap,
                    Zeroizing::new(vec![0x61; crate::MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES + 1]),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &[0xff],
                    Zeroizing::new(b"separate export passphrase".to_vec()),
                    policy,
                    randomness(),
                )
                .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"separate export passphrase".to_vec()),
                policy,
                randomness(),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"separate export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 0);
        assert_eq!(opened.candidate_count(), 0);
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(Vec::new()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(
            crate::open_portable_with_passphrase(
                artifact.as_bytes(),
                Zeroizing::new(vec![0x61; crate::MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES + 1]),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );

        let CborValue::Map(mut unsupported_version) = decode_cbor(artifact.as_bytes()).unwrap()
        else {
            panic!()
        };
        replace_cbor_field(&mut unsupported_version, 1, CborValue::Unsigned(2));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(unsupported_version)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::Unsupported)
        );

        let CborValue::Map(mut excessive_kdf) = decode_cbor(artifact.as_bytes()).unwrap() else {
            panic!()
        };
        let CborValue::Map(kdf_fields) = excessive_kdf
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(4))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        replace_cbor_field(kdf_fields, 1, CborValue::Unsigned(16 * 1024));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(excessive_kdf)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::BoundExceeded)
        );

        let CborValue::Map(mut unknown_field) = decode_cbor(artifact.as_bytes()).unwrap() else {
            panic!()
        };
        unknown_field.push((CborValue::Unsigned(8), CborValue::Null));
        assert_eq!(
            crate::open_portable_with_passphrase(
                &encode_cbor(&CborValue::Map(unknown_field)),
                Zeroizing::new(b"separate export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_export_preserves_every_current_conflict_tombstone() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x7d; 16]);
        let (publication, expected_revision_ids) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        let artifact = session
            .export_portable_with_passphrase(
                &exact_bootstrap,
                Zeroizing::new(b"conflict export passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0x7e; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"conflict export passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(opened.item_count(), 1);
        assert_eq!(opened.candidate_count(), 2);
        let plaintext = crate::export::decrypt_portable_for_test(
            artifact.as_bytes(),
            Zeroizing::new(b"conflict export passphrase".to_vec()),
        )
        .unwrap();
        let CborValue::Map(mut snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let mut entries = take_cbor_field(&mut snapshot, 3);
        assert_eq!(take_cbor_field(&mut snapshot, 4), CborValue::Unsigned(2));
        let CborValue::Array(exported_candidates) = &mut entries else {
            panic!()
        };
        let mut actual_revision_ids = Vec::new();
        for value in exported_candidates.drain(..) {
            let CborValue::Map(mut entry) = value else {
                panic!()
            };
            assert_eq!(
                take_cbor_field(&mut entry, 1),
                CborValue::Bytes(item_id.as_bytes().to_vec())
            );
            let CborValue::Bytes(revision_id) = take_cbor_field(&mut entry, 2) else {
                panic!()
            };
            let revision_id = RevisionId::new(revision_id.try_into().unwrap());
            actual_revision_ids.push(revision_id);
            let CborValue::Bytes(mut encoded_revision) = take_cbor_field(&mut entry, 3) else {
                panic!()
            };
            let candidate = crate::decode_item_revision(revision_id, &encoded_revision).unwrap();
            assert!(matches!(candidate.state(), ItemState::Tombstone(_)));
            encoded_revision.zeroize();
        }
        assert_eq!(actual_revision_ids, expected_revision_ids);
        crate::export::zeroize_cbor(&mut entries);

        let CborValue::Map(mut reversed_snapshot) = decode_cbor(&plaintext).unwrap() else {
            panic!()
        };
        let CborValue::Array(candidate_entries) = reversed_snapshot
            .iter_mut()
            .find(|(key, _)| key == &CborValue::Unsigned(3))
            .map(|(_, value)| value)
            .unwrap()
        else {
            panic!()
        };
        candidate_entries.reverse();
        refresh_portable_snapshot_hash(&mut reversed_snapshot);
        let reversed_snapshot =
            authenticate_portable_snapshot(reversed_snapshot, b"conflict export passphrase", 0x7f);
        assert_eq!(
            crate::open_portable_with_passphrase(
                &reversed_snapshot,
                Zeroizing::new(b"conflict export passphrase".to_vec()),
                crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn portable_import_rekeys_an_independently_reopenable_new_vault() {
        let (source_locator, source_local, source_bootstrap, source_factory) = initialized();
        let exact_active = source_local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(source_active) =
            LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let conflicted_source_item = ItemId::new([0x7d; 16]);
        let (publication, _) = pending_tombstone_publication(
            &source_active,
            conflicted_source_item,
            conflicted_source_item,
            2,
            None,
        );
        *source_local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(source_active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let live_randomness = add_item_randomness(0x9a);
        let live_source_item = live_randomness.item_id();
        source_session
            .add_item(
                new_login_document(live_source_item, "Restored portal", "restored-secret"),
                900,
                live_randomness,
                &source_local,
            )
            .unwrap();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        assert_eq!(source_session.item_count(), 2);
        assert_eq!(source_session.candidate_count(), 3);
        assert_eq!(source_session.conflicted_item_count(), 1);
        let source_vault_id = source_session.vault_id();
        let source_item_ids = source_session
            .current_catalog
            .items
            .keys()
            .copied()
            .collect::<BTreeSet<_>>();
        let source_revision_ids = source_session
            .current_catalog
            .items
            .values()
            .flatten()
            .map(ItemCandidate::revision_id)
            .collect::<BTreeSet<_>>();
        let exact_source_bootstrap = source_bootstrap.latest_bytes().unwrap();
        let artifact = source_session
            .export_portable_with_passphrase(
                &exact_source_bootstrap,
                Zeroizing::new(b"restore passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0x8b; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let restore_expectation = opened.prepare_restore_verification().unwrap();
        assert_eq!(
            format!("{restore_expectation:?}"),
            "PortableRestoreExpectationV1(<redacted>)"
        );
        let import_random_byte_count = crate::portable_import_random_bytes(&opened).unwrap();
        assert_eq!(import_random_byte_count, 2 * 16 + 6 * 80 + 32);
        assert_eq!(
            crate::PortableImportRandomnessV1::new(
                vec![0x91; import_random_byte_count - 1],
                &opened,
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
        let import_randomness_bytes = (0..import_random_byte_count)
            .map(|index| (index as u8).wrapping_mul(43).wrapping_add(0x91))
            .collect::<Vec<_>>();
        let import_randomness =
            crate::PortableImportRandomnessV1::new(import_randomness_bytes.clone(), &opened)
                .unwrap();
        assert_eq!(
            format!("{import_randomness:?}"),
            "PortableImportRandomnessV1(<redacted>)"
        );

        let mut target_generation_zero = generation_zero_bytes();
        for byte in &mut target_generation_zero {
            *byte = byte.wrapping_add(0x65);
        }
        let (target_locator, target_local, target_bootstrap, target_factory) = initialized_with(
            b"independent target passphrase",
            GenerationZeroRandomness::new(target_generation_zero),
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_ne!(target_session.vault_id(), source_vault_id);
        target_session
            .activate_audit_epoch(900, audited_access_randomness(0x89), &target_local)
            .unwrap();
        let target_session = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        target_local.fail_next_compare(LocalStateStoreError::Unavailable);
        assert_eq!(
            target_session
                .import_opened_portable_snapshot(opened, 901, import_randomness, &target_local)
                .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(target_session.item_count(), 0);
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let import_randomness =
            crate::PortableImportRandomnessV1::new(import_randomness_bytes, &opened).unwrap();
        let imported_active = target_session
            .import_opened_portable_snapshot(opened, 901, import_randomness, &target_local)
            .unwrap();
        assert_eq!(imported_active.last_device_counter(), 3);

        drop(source_session);
        let restored = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(restored.item_count(), 2);
        assert_eq!(restored.candidate_count(), 3);
        assert_eq!(restored.conflicted_item_count(), 1);
        assert!(restored
            .current_catalog
            .items
            .keys()
            .all(|item_id| !source_item_ids.contains(item_id)));
        assert!(restored
            .current_catalog
            .items
            .values()
            .flatten()
            .all(|candidate| {
                !source_revision_ids.contains(&candidate.revision_id())
                    && candidate.causal_parents().is_empty()
            }));
        let mut tombstone_times = Vec::new();
        let mut restored_login = None;
        for candidates in restored.current_catalog.items.values() {
            for candidate in candidates {
                assert_eq!(candidate.item_id(), candidate.state().item_id());
                match candidate.state() {
                    ItemState::Live(document) => {
                        let AnyRecord::Login(login) = document.payload() else {
                            panic!("fixture must retain a login")
                        };
                        restored_login = Some((login.title.clone(), login.password.clone()));
                    }
                    ItemState::Tombstone(tombstone) => {
                        tombstone_times.push(tombstone.deleted_at_ms);
                    }
                }
            }
        }
        tombstone_times.sort_unstable();
        assert_eq!(tombstone_times, vec![100, 101]);
        assert_eq!(
            restored_login,
            Some(("Restored portal".to_owned(), "restored-secret".to_owned()))
        );

        let mismatched_expectation = crate::PortableRestoreExpectationV1::from_source(
            VaultId::new([0xe9; 16]),
            &BTreeMap::new(),
        )
        .unwrap();
        let failed = restored
            .audited_verify_portable_restore(
                mismatched_expectation,
                902,
                audited_access_randomness(0x8a),
                &target_local,
            )
            .unwrap();
        assert!(!failed.operation_succeeded());
        assert_eq!(
            failed.into_operation(),
            Err(ApplicationError::IntegrityFailure)
        );
        let restored = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&restored),
            (
                AuditActionV1::PortableRestoreVerify,
                AuditOutcomeV1::Failed,
                None,
                None,
            )
        );
        let verified = restored
            .audited_verify_portable_restore(
                restore_expectation,
                903,
                audited_access_randomness(0x8b),
                &target_local,
            )
            .unwrap();
        assert!(verified.operation_succeeded());
        let report = verified.into_operation().unwrap();
        assert_eq!(report.item_count(), 2);
        assert_eq!(report.candidate_count(), 3);
        assert_eq!(report.conflicted_item_count(), 1);
        assert_eq!(
            format!("{report:?}"),
            "PortableRestoreVerificationV1 { item_count: 2, candidate_count: 3, conflicted_item_count: 1 }"
        );
        let restored = open_active_vault(
            Zeroizing::new(b"independent target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&restored),
            (
                AuditActionV1::PortableRestoreVerify,
                AuditOutcomeV1::Succeeded,
                None,
                None,
            )
        );
        let audit = restored.audit_verify().unwrap();
        assert_eq!(audit.commit_count(), 5);
        assert_eq!(audit.catalog_count(), 2);
        assert_eq!(audit.revision_count(), 3);
        assert_eq!(audit.item_count(), 2);
        assert_eq!(audit.audit_event_count(), 4);
    }

    #[test]
    fn portable_restore_verification_refuses_pre_audit_target_without_comparison() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let expectation = crate::PortableRestoreExpectationV1::from_source(
            VaultId::new([0xe8; 16]),
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(matches!(
            session.audited_verify_portable_restore(
                expectation,
                904,
                audited_access_randomness(0x8c),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        ));
        assert_eq!(*local.0.lock().unwrap(), Some(exact_active));
    }

    #[test]
    fn portable_import_rejects_source_and_mutated_targets_before_local_writes() {
        let (source_locator, source_local, source_bootstrap, source_factory) = initialized();
        let source_session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            source_locator,
            &source_local,
            &source_bootstrap,
            &source_factory,
        )
        .unwrap();
        let exact_source_bootstrap = source_bootstrap.latest_bytes().unwrap();
        let artifact = source_session
            .export_portable_with_passphrase(
                &exact_source_bootstrap,
                Zeroizing::new(b"empty restore passphrase".to_vec()),
                crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                crate::PortableExportRandomnessV1::new([0xa2; crate::PORTABLE_EXPORT_RANDOM_BYTES]),
            )
            .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"empty restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        assert_eq!(crate::portable_import_random_bytes(&opened), Ok(272));
        let randomness = crate::PortableImportRandomnessV1::new(vec![0xa3; 272], &opened).unwrap();
        let source_compare_calls = source_local.3.load(Ordering::SeqCst);
        assert_eq!(
            source_session
                .import_opened_portable_snapshot(opened, 902, randomness, &source_local)
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(source_local.3.load(Ordering::SeqCst), source_compare_calls);

        let mut target_generation_zero = generation_zero_bytes();
        for byte in &mut target_generation_zero {
            *byte = byte.wrapping_add(0x43);
        }
        let (target_locator, target_local, target_bootstrap, target_factory) = initialized_with(
            b"mutated target passphrase",
            GenerationZeroRandomness::new(target_generation_zero),
        );
        let target_session = open_active_vault(
            Zeroizing::new(b"mutated target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        let add_randomness = add_item_randomness(0xa4);
        let target_item_id = add_randomness.item_id();
        target_session
            .add_item(
                new_login_document(target_item_id, "Existing target", "target-secret"),
                903,
                add_randomness,
                &target_local,
            )
            .unwrap();
        let target_session = open_active_vault(
            Zeroizing::new(b"mutated target passphrase".to_vec()),
            target_locator,
            &target_local,
            &target_bootstrap,
            &target_factory,
        )
        .unwrap();
        let opened = crate::open_portable_with_passphrase(
            artifact.as_bytes(),
            Zeroizing::new(b"empty restore passphrase".to_vec()),
            crate::PortableOpenPolicyV1::new(8 * 1024, 1, 1).unwrap(),
        )
        .unwrap();
        let randomness = crate::PortableImportRandomnessV1::new(vec![0xa5; 272], &opened).unwrap();
        let target_compare_calls = target_local.3.load(Ordering::SeqCst);
        assert_eq!(
            target_session
                .import_opened_portable_snapshot(opened, 904, randomness, &target_local)
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(target_local.3.load(Ordering::SeqCst), target_compare_calls);
    }

    #[test]
    fn audit_verify_rejects_a_local_counter_without_an_exact_pinned_anchor() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        session.active = ActiveStateV1::new(
            session.active.bootstrap_locator(),
            session.active.vault_id(),
            session.active.bootstrap_id(),
            session.active.authority_fingerprint(),
            session.active.device_id(),
            session.active.device_certificate_id(),
            session.active.device_certificate_frame().clone(),
            session.active.local_secret().clone(),
            session.active.pinned_heads().clone(),
            session.active.last_device_counter() + 1,
            session.active.catalog_root(),
        )
        .unwrap();
        assert_eq!(
            session.audit_verify(),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn lifecycle_retains_locked_state_on_failure_and_drops_session_on_lock() {
        let (locator, local, bootstrap, factory) = initialized();
        let locked = crate::LockedVaultV1::new(locator);
        assert_eq!(locked.locator(), locator);
        assert_eq!(format!("{locked:?}"), "LockedVaultV1(<locked>)");

        let mut access = crate::VaultAccessV1::locked(locator);
        assert!(access.is_locked());
        assert!(!access.is_unlocked());
        assert!(matches!(
            access.as_unlocked(),
            Err(ApplicationError::Locked)
        ));
        assert_eq!(format!("{access:?}"), "VaultAccessV1::Locked(<redacted>)");

        assert_eq!(
            access.unlock(
                Zeroizing::new(b"wrong".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::AuthenticationFailed)
        );
        assert!(access.is_locked());

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert!(access.is_unlocked());
        assert_eq!(access.as_unlocked().unwrap().item_count(), 0);
        assert_eq!(format!("{access:?}"), "VaultAccessV1::Unlocked(<redacted>)");
        assert_eq!(
            access.unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert!(access.is_unlocked());

        access.lock();
        assert!(access.is_locked());
        access.lock();
        assert!(matches!(
            access.into_unlocked(),
            Err(ApplicationError::Locked)
        ));

        let mut access = crate::VaultAccessV1::locked(locator);
        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(access.into_unlocked().unwrap().item_count(), 0);
    }

    #[test]
    fn status_is_safe_while_locked_and_adds_counts_only_while_unlocked() {
        let absent_local = MemoryLocalStateStore::default();
        let absent = crate::VaultAccessV1::locked(BootstrapLocator::new([0x91; 32]));
        let status = absent.status(&absent_local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Absent);
        assert_eq!(status.item_count(), None);
        assert_eq!(status.candidate_count(), None);
        assert_eq!(status.conflicted_item_count(), None);
        assert_eq!(format!("{status:?}"), "VaultStatusV1 { state: Absent }");

        let prepared = prepare_generation_zero(
            Zeroizing::new(b"prepared passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let prepared_locator = prepared.bootstrap_locator();
        let prepared_local =
            MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        let status = crate::VaultAccessV1::locked(prepared_locator)
            .status(&prepared_local)
            .unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Prepared);
        assert_eq!(status.item_count(), None);

        let (locator, local, bootstrap, factory) = initialized();
        let mut access = crate::VaultAccessV1::locked(locator);
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Locked);
        assert_eq!(status.item_count(), None);

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::Unlocked);
        assert_eq!(status.item_count(), Some(0));
        assert_eq!(status.candidate_count(), Some(0));
        assert_eq!(status.conflicted_item_count(), Some(0));
        assert_eq!(
            format!("{status:?}"),
            "VaultStatusV1 { state: Unlocked, item_count: 0, candidate_count: 0, conflicted_item_count: 0 }"
        );

        access.lock();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        let status = access.status(&local).unwrap();
        assert_eq!(status.state(), crate::VaultStatusStateV1::RecoveryRequired);
        assert_eq!(status.item_count(), None);
        assert_eq!(
            format!("{status:?}"),
            "VaultStatusV1 { state: RecoveryRequired }"
        );
    }

    #[test]
    fn doctor_reports_coarse_locked_and_unlocked_health_states() {
        let absent_local = MemoryLocalStateStore::default();
        let absent_bootstrap = MemoryBootstrapStore::default();
        let absent = crate::VaultAccessV1::locked(BootstrapLocator::new([0x93; 32]));
        assert_eq!(
            absent.doctor(&absent_local, &absent_bootstrap).state(),
            crate::VaultDoctorStateV1::InitializationRequired
        );

        let prepared = prepare_generation_zero(
            Zeroizing::new(b"prepared passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let prepared_locator = prepared.bootstrap_locator();
        let prepared_local =
            MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        assert_eq!(
            crate::VaultAccessV1::locked(prepared_locator)
                .doctor(&prepared_local, &absent_bootstrap)
                .state(),
            crate::VaultDoctorStateV1::InitializationRequired
        );

        let (locator, local, bootstrap, factory) = initialized();
        let mut access = crate::VaultAccessV1::locked(locator);
        let locked_report = access.doctor(&local, &bootstrap);
        assert_eq!(
            locked_report.state(),
            crate::VaultDoctorStateV1::AuthenticationRequired
        );
        assert_eq!(
            format!("{locked_report:?}"),
            "VaultDoctorReportV1 { state: AuthenticationRequired }"
        );

        access
            .unlock(
                Zeroizing::new(b"active passphrase".to_vec()),
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
        assert_eq!(
            access.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::Healthy
        );

        access.lock();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        assert_eq!(
            access.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::RecoveryRequired
        );
    }

    #[test]
    fn doctor_closes_store_version_and_integrity_failures_without_detail() {
        struct FailingLocalStateStore(LocalStateStoreError);

        impl LocalStateStore for FailingLocalStateStore {
            fn load(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
                Err(self.0)
            }

            fn compare_exchange(
                &self,
                _locator: BootstrapLocator,
                _expected: Option<&[u8]>,
                _replacement: &[u8],
            ) -> Result<(), LocalStateStoreError> {
                Err(self.0)
            }
        }

        struct FailingBootstrapStore(BootstrapStoreError);

        impl BootstrapStore for FailingBootstrapStore {
            fn load_latest(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
                Err(self.0)
            }

            fn put_generation(
                &self,
                _locator: BootstrapLocator,
                _expected_previous: Option<BootstrapId>,
                _exact_bootstrap: &[u8],
            ) -> Result<(), BootstrapStoreError> {
                Err(self.0)
            }

            fn supersede_generation(
                &self,
                _locator: BootstrapLocator,
                _superseded: BootstrapId,
            ) -> Result<(), BootstrapStoreError> {
                Err(self.0)
            }
        }

        let locator = BootstrapLocator::new([0x94; 32]);
        let access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .doctor(
                    &FailingLocalStateStore(LocalStateStoreError::Unavailable),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::LocalStateUnavailable
        );
        assert_eq!(
            access
                .doctor(
                    &FailingLocalStateStore(LocalStateStoreError::ConcurrentHost),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(
                    &MemoryLocalStateStore::with_state(vec![0xff]),
                    &MemoryBootstrapStore::default(),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        let (locator, local, bootstrap, _) = initialized();
        let access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .doctor(
                    &local,
                    &FailingBootstrapStore(BootstrapStoreError::Unavailable),
                )
                .state(),
            crate::VaultDoctorStateV1::BootstrapUnavailable
        );
        assert_eq!(
            access
                .doctor(
                    &local,
                    &FailingBootstrapStore(BootstrapStoreError::Conflict),
                )
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(&local, &MemoryBootstrapStore::default())
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        assert_eq!(
            access
                .doctor(&local, &MemoryBootstrapStore::with_latest(vec![0xff]),)
                .state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        let unsupported_local =
            replace_top_level_version(local.0.lock().unwrap().as_deref().unwrap(), 2);
        let unsupported_local = MemoryLocalStateStore::with_state(unsupported_local);
        assert_eq!(
            access.doctor(&unsupported_local, &bootstrap).state(),
            crate::VaultDoctorStateV1::UnsupportedCapability
        );

        let unsupported_bootstrap =
            replace_top_level_version(&bootstrap.latest_bytes().unwrap(), 2);
        let unsupported_bootstrap = MemoryBootstrapStore::with_latest(unsupported_bootstrap);
        assert_eq!(
            access.doctor(&local, &unsupported_bootstrap).state(),
            crate::VaultDoctorStateV1::UnsupportedCapability
        );
    }

    #[test]
    fn unlocked_doctor_distinguishes_repository_unavailability_from_integrity() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut unavailable = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        unavailable._repository = Box::new(FailingAuditRepository(
            ApplicationRepositoryError::StorageUnavailable,
        ));
        let unavailable = crate::VaultAccessV1::Unlocked(Box::new(unavailable));
        assert_eq!(
            unavailable.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::RepositoryUnavailable
        );

        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("initialized state must be active")
        };
        let pending =
            LocalVaultStateV1::pending_publication(active.clone(), pending_publication(&active))
                .unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        assert_eq!(
            unavailable.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
        *local.0.lock().unwrap() = Some(exact_active);

        let mut corrupt = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        corrupt._repository = Box::new(FailingAuditRepository(
            ApplicationRepositoryError::IntegrityFailure,
        ));
        let corrupt = crate::VaultAccessV1::Unlocked(Box::new(corrupt));
        assert_eq!(
            corrupt.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );

        *local.0.lock().unwrap() = None;
        assert_eq!(
            corrupt.doctor(&local, &bootstrap).state(),
            crate::VaultDoctorStateV1::IntegrityFailure
        );
    }

    #[test]
    fn locked_status_closes_owner_state_failures() {
        struct FailingLocalStateStore(LocalStateStoreError);

        impl LocalStateStore for FailingLocalStateStore {
            fn load(
                &self,
                _locator: BootstrapLocator,
            ) -> Result<Option<Vec<u8>>, LocalStateStoreError> {
                Err(self.0)
            }

            fn compare_exchange(
                &self,
                _locator: BootstrapLocator,
                _expected: Option<&[u8]>,
                _replacement: &[u8],
            ) -> Result<(), LocalStateStoreError> {
                Err(self.0)
            }
        }

        let access = crate::VaultAccessV1::locked(BootstrapLocator::new([0x92; 32]));
        for (store_error, expected) in [
            (
                LocalStateStoreError::Unavailable,
                ApplicationError::StorageUnavailable,
            ),
            (
                LocalStateStoreError::ConcurrentHost,
                ApplicationError::ConcurrentHost,
            ),
            (
                LocalStateStoreError::Corruption,
                ApplicationError::IntegrityFailure,
            ),
        ] {
            assert_eq!(
                access.status(&FailingLocalStateStore(store_error)),
                Err(expected)
            );
        }

        let corrupt = MemoryLocalStateStore::with_state(vec![0xff]);
        assert_eq!(
            access.status(&corrupt),
            Err(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_materializes_every_current_revision_candidate() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x21; 16]);
        let (publication, expected_revision_ids) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.item_count(), 1);
        assert_eq!(session.candidate_count(), 2);
        assert_eq!(session.conflicted_item_count(), 1);
        let candidates = session.current_catalog.items.get(&item_id).unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(ItemCandidate::revision_id)
                .collect::<Vec<_>>(),
            expected_revision_ids
        );
        assert!(format!("{session:?}").contains("item_count: 1"));
        assert!(!format!("{session:?}").contains(&item_id.to_user_string()));
    }

    #[test]
    fn current_item_reads_return_only_typed_redacted_views() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x27; 16]);
        let title = "ÉCLAIR Personal portal";
        let password = "never-log-this-password";
        let publication = pending_live_publication(&active, item_id, title, password);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let view = session.get_item(item_id).unwrap().unwrap();
        assert_eq!(view.item_id, item_id);
        assert_eq!(view.schema.as_str(), LOGIN_V1);
        match &view.record {
            RedactedRecordView::Login {
                title: view_title,
                username,
                urls,
                password: redacted_password,
                has_notes,
            } => {
                assert_eq!(view_title, title);
                assert_eq!(username, "ada@example.test");
                assert_eq!(urls, &["https://example.test"]);
                assert_eq!(redacted_password.to_string(), "<redacted>");
                assert!(*has_notes);
            }
            _ => panic!("fixture must project as a login"),
        }
        assert_eq!(session.list_items().unwrap(), vec![view.clone()]);
        assert_eq!(
            session.current_item_revision(item_id).unwrap(),
            Some(session.current_catalog.items[&item_id][0].revision_id())
        );
        assert_eq!(session.get_item(ItemId::new([0x28; 16])).unwrap(), None);
        assert_eq!(
            session
                .current_item_revision(ItemId::new([0x28; 16]))
                .unwrap(),
            None
        );
        assert_eq!(session.search_item_count(), 1);
        for query in [
            "E\u{301}CLAIR",
            "ada@EXAMPLE",
            "AMPLE.TEST",
            "finance",
            "portal ada@example",
            "a",
        ] {
            assert_eq!(
                session
                    .search_items(Zeroizing::new(query.to_owned()), None, 10)
                    .unwrap(),
                vec![view.clone()]
            );
        }
        assert!(session
            .search_items(Zeroizing::new("   ".to_owned()), None, 10)
            .unwrap()
            .is_empty());
        assert_eq!(
            session
                .search_items(
                    Zeroizing::new("portal".to_owned()),
                    Some(CollectionId::new([0x82; 16])),
                    10,
                )
                .unwrap(),
            vec![view.clone()]
        );
        assert!(session
            .search_items(
                Zeroizing::new("portal".to_owned()),
                Some(CollectionId::new([0x85; 16])),
                10,
            )
            .unwrap()
            .is_empty());
        for secret in [password, "private note"] {
            assert!(session
                .search_items(Zeroizing::new(secret.to_owned()), None, 10)
                .unwrap()
                .is_empty());
        }
        for invalid in ["", "line\nbreak", "\0"] {
            assert_eq!(
                session.search_items(Zeroizing::new(invalid.to_owned()), None, 10),
                Err(ApplicationError::InvalidInput)
            );
        }
        assert_eq!(
            session.search_items(Zeroizing::new("x".repeat(257)), None, 10),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.search_items(Zeroizing::new("portal".to_owned()), None, 0),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.search_items(Zeroizing::new("portal".to_owned()), None, 10_001),
            Err(ApplicationError::BoundExceeded)
        );
        let debug = format!("{view:?}");
        assert!(!debug.contains(title));
        assert!(!debug.contains(password));
        assert!(!debug.contains(&item_id.to_user_string()));
    }

    #[test]
    fn current_item_reads_hide_tombstones_and_fail_closed_on_conflicts() {
        for candidate_count in [1, 2] {
            let (locator, local, bootstrap, factory) = initialized();
            let exact_active = local.0.lock().unwrap().clone().unwrap();
            let LocalVaultStateV1::Active(active) =
                LocalVaultStateV1::decode(&exact_active).unwrap()
            else {
                panic!("fixture must be active")
            };
            let item_id = ItemId::new([0x29; 16]);
            let (publication, _) =
                pending_tombstone_publication(&active, item_id, item_id, candidate_count, None);
            let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
            *local.0.lock().unwrap() = Some(pending.encode().unwrap());
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            let mut session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

            if candidate_count == 1 {
                assert_eq!(session.get_item(item_id).unwrap(), None);
                assert_eq!(session.current_item_revision(item_id).unwrap(), None);
                assert!(session.list_items().unwrap().is_empty());
            } else {
                assert_eq!(
                    session.get_item(item_id),
                    Err(ApplicationError::ConflictRequired)
                );
                assert_eq!(
                    session.list_items(),
                    Err(ApplicationError::ConflictRequired)
                );
                assert_eq!(
                    session.current_item_revision(item_id),
                    Err(ApplicationError::ConflictRequired)
                );
                assert_eq!(session.candidate_count(), 2);
                assert_eq!(
                    session.search_items(Zeroizing::new("anything".to_owned()), None, 10),
                    Err(ApplicationError::ConflictRequired)
                );
                activate_audit_epoch_for_test(
                    &session.active,
                    &session._keys,
                    &session._local_secret,
                    session._repository.as_ref(),
                    297,
                    None,
                    None,
                    [0x30; AUDIT_ONLY_TEST_RANDOM_BYTES],
                    &local,
                )
                .unwrap();
                drop(session);
                let conflict = open_active_vault(
                    Zeroizing::new(b"active passphrase".to_vec()),
                    locator,
                    &local,
                    &bootstrap,
                    &factory,
                )
                .unwrap()
                .audited_search_items(
                    Zeroizing::new("anything".to_owned()),
                    None,
                    10,
                    298,
                    audited_access_randomness(0x32),
                    &local,
                )
                .unwrap();
                assert_eq!(
                    conflict.into_operation(),
                    Err(ApplicationError::ConflictRequired)
                );
                session = open_active_vault(
                    Zeroizing::new(b"active passphrase".to_vec()),
                    locator,
                    &local,
                    &bootstrap,
                    &factory,
                )
                .unwrap();
                assert_eq!(
                    latest_audit_facts(&session),
                    (
                        AuditActionV1::ItemSearch,
                        AuditOutcomeV1::Failed,
                        None,
                        None,
                    )
                );
            }
            let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
            let exact_state = local.0.lock().unwrap().clone().unwrap();
            assert_eq!(
                session.replace_item(
                    expected_revision,
                    new_login_document(item_id, "Cannot replace", "secret"),
                    299,
                    replace_item_randomness(0x31),
                    &local,
                ),
                Err(ApplicationError::ConflictRequired)
            );
            assert_eq!(
                local.0.lock().unwrap().as_deref(),
                Some(exact_state.as_slice())
            );
        }
    }

    #[test]
    fn conflict_candidates_are_redacted_and_choose_resolution_retains_every_parent() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2a; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let views = session.conflict_candidates(item_id).unwrap();
        assert_eq!(
            views
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<Vec<_>>()
        );
        let titles = views
            .iter()
            .map(|view| {
                let RedactedRecordView::Login { title, .. } = &view.redacted_item().unwrap().record
                else {
                    panic!("conflict fixture must contain logins")
                };
                title.as_str()
            })
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Keep left"));
        assert!(titles.contains(&"Keep right"));
        let debug = format!("{views:?}");
        for hidden in [
            "Keep left",
            "Keep right",
            "left-secret",
            "right-secret",
            &item_id.to_user_string(),
        ] {
            assert!(!debug.contains(hidden));
        }

        let selected_revision = revisions
            .iter()
            .find(|(_, title)| title == "Keep right")
            .map(|(revision_id, _)| *revision_id)
            .unwrap();
        let prior_heads = session.local_pins().clone();
        let resolved = session
            .resolve_item_conflict(
                selected_revision,
                401,
                resolve_item_conflict_randomness(0x4d),
                &local,
            )
            .unwrap();
        assert_eq!(resolved.last_device_counter(), 3);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("resolution must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("selected live conflict candidate must remain live")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("selected conflict candidate must retain its schema")
        };
        assert_eq!(login.title, "Keep right");
        assert_eq!(login.password, "right-secret");
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 401);
    }

    #[test]
    fn audited_item_bound_conflict_resolution_records_failure_and_atomic_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2b; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(402, audited_access_randomness(0x4e), &local)
        .unwrap();

        let missing_revision = RevisionId::new([0xff; 32]);
        let failed = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .audited_resolve_item_conflict_for_item(
            item_id,
            missing_revision,
            403,
            resolve_item_conflict_randomness(0x4f),
            audited_access_randomness(0x50),
            &local,
        )
        .unwrap();
        assert_eq!(failed.into_operation(), Err(ApplicationError::NotFound));
        let after_failure = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(after_failure.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&after_failure),
            (
                AuditActionV1::ItemConflictResolve,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );

        let selected_revision = revisions[1].0;
        after_failure
            .audited_resolve_item_conflict_for_item(
                item_id,
                selected_revision,
                404,
                resolve_item_conflict_randomness(0x51),
                audited_access_randomness(0x52),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();
        let resolved = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(resolved.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&resolved),
            (
                AuditActionV1::ItemConflictResolve,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(selected_revision),
            )
        );
    }

    #[test]
    fn authored_conflict_merge_publishes_complete_parent_set_and_document() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x25; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        session
            .merge_item_conflict(
                new_login_document(item_id, "Merged result", "merged-secret"),
                405,
                resolve_item_conflict_randomness(0x51),
                &local,
            )
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored merge must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored merge must publish a live document")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("authored merge must retain the input schema")
        };
        assert_eq!(login.title, "Merged result");
        assert_eq!(login.password, "merged-secret");
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 405);
    }

    #[test]
    fn audited_authored_login_merge_records_failures_and_all_current_parent_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x24; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(406, audited_access_randomness(0x56), &local)
        .unwrap();

        let missing = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_login_conflict_merge(
            item_id,
            RevisionId::new([0xff; 32]),
            407,
            audited_access_randomness(0x57),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(missing, Err(ApplicationError::NotFound)));
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );

        let base_revision = revisions[0].0;
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .expect("selected base must remain a live current candidate");
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_login_conflict_merge(
                item_id,
                base_revision,
                408,
                audited_access_randomness(0x58),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .record_audited_host_failure(&local)
            .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );

        let invalid = session
            .prepare_audited_login_conflict_merge(
                item_id,
                base_revision,
                409,
                audited_access_randomness(0x59),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                LoginEditInputV1::new(
                    Zeroizing::new("Too many URLs".to_owned()),
                    Zeroizing::new("merge@example.test".to_owned()),
                    Zeroizing::new("invalid-merged-secret".to_owned()),
                    (0..17)
                        .map(|index| {
                            Zeroizing::new(format!("https://{index}.invalid.example.test"))
                        })
                        .collect(),
                    None,
                ),
                resolve_item_conflict_randomness(0x5a),
                &local,
            )
            .unwrap();
        assert_eq!(
            invalid.into_operation(),
            Err(ApplicationError::InvalidInput)
        );
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );

        session
            .prepare_audited_login_conflict_merge(
                item_id,
                base_revision,
                410,
                audited_access_randomness(0x5b),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                LoginEditInputV1::new(
                    Zeroizing::new("Authored merge".to_owned()),
                    Zeroizing::new("merged@example.test".to_owned()),
                    Zeroizing::new("authored-merged-secret".to_owned()),
                    vec![
                        Zeroizing::new("https://merged.example.test".to_owned()),
                        Zeroizing::new("https://backup.example.test".to_owned()),
                    ],
                    Some(Zeroizing::new("authored private notes".to_owned())),
                ),
                resolve_item_conflict_randomness(0x5c),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored login merge must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions
                .iter()
                .map(|(revision_id, _)| *revision_id)
                .collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored login merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 300);
        assert_eq!(document.updated_at_ms(), 410);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::Login(login) = document.payload() else {
            panic!("authored login merge must retain the login schema")
        };
        assert_eq!(login.title, "Authored merge");
        assert_eq!(login.username, "merged@example.test");
        assert_eq!(login.password, "authored-merged-secret");
        assert_eq!(
            login.urls,
            ["https://merged.example.test", "https://backup.example.test"]
        );
        assert_eq!(login.notes.as_deref(), Some("authored private notes"));
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_login_merge_rejects_a_current_tombstone_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x23; 16]);
        let (publication, revisions) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(411, audited_access_randomness(0x5d), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_login_conflict_merge(
            item_id,
            revisions[0],
            412,
            audited_access_randomness(0x5e),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::InvalidInput)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_secure_note_merge_records_host_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x22; 16]);
        let (publication, revisions) = pending_secure_note_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(413, audited_access_randomness(0x5f), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_secure_note_conflict_merge(
            item_id,
            base_revision,
            414,
            audited_access_randomness(0x60),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_secure_note_conflict_merge(
                item_id,
                base_revision,
                415,
                audited_access_randomness(0x61),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                SecureNoteConflictMergeInputV1::new(
                    Zeroizing::new("Authored secure note".to_owned()),
                    Zeroizing::new("authored secure-note body".to_owned()),
                ),
                resolve_item_conflict_randomness(0x62),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored secure-note merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored secure-note merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 300);
        assert_eq!(document.updated_at_ms(), 415);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::SecureNote(note) = document.payload() else {
            panic!("authored secure-note merge must retain its schema")
        };
        assert_eq!(note.title, "Authored secure note");
        assert_eq!(note.body, "authored secure-note body");
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_card_merge_records_host_validation_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x24; 16]);
        let (publication, revisions) = pending_card_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(501, audited_access_randomness(0x65), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_card_conflict_merge(
            item_id,
            base_revision,
            502,
            audited_access_randomness(0x66),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        session
            .prepare_audited_card_conflict_merge(
                item_id,
                base_revision,
                503,
                audited_access_randomness(0x67),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                CardConflictMergeInputV1::new(
                    Zeroizing::new("Authored card".to_owned()),
                    Zeroizing::new("Ada Lovelace".to_owned()),
                    Zeroizing::new("not-a-pan".to_owned()),
                    Zeroizing::new("12".to_owned()),
                    Zeroizing::new("2032".to_owned()),
                    Zeroizing::new("999".to_owned()),
                    None,
                ),
                resolve_item_conflict_randomness(0x68),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("invalid card form must remain a closed failed operation");
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_card_conflict_merge(
                item_id,
                base_revision,
                504,
                audited_access_randomness(0x69),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                CardConflictMergeInputV1::new(
                    Zeroizing::new("Authored card".to_owned()),
                    Zeroizing::new("Grace Hopper".to_owned()),
                    Zeroizing::new("4000000000000002".to_owned()),
                    Zeroizing::new("7".to_owned()),
                    Zeroizing::new("2032".to_owned()),
                    Zeroizing::new("999".to_owned()),
                    Some(Zeroizing::new("90210".to_owned())),
                ),
                resolve_item_conflict_randomness(0x6a),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored payment-card merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored payment-card merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 500);
        assert_eq!(document.updated_at_ms(), 504);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::Card(card) = document.payload() else {
            panic!("authored payment-card merge must retain its schema")
        };
        assert_eq!(card.title, "Authored card");
        assert_eq!(card.holder, "Grace Hopper");
        assert_eq!(card.number, "4000000000000002");
        assert_eq!(card.expiry_month, 7);
        assert_eq!(card.expiry_year, 2032);
        assert_eq!(card.cvv, "999");
        assert_eq!(card.billing_zip.as_deref(), Some("90210"));
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_card_merge_rejects_a_secure_note_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x25; 16]);
        let (publication, revisions) = pending_secure_note_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(505, audited_access_randomness(0x6b), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_card_conflict_merge(
            item_id,
            revisions[0],
            506,
            audited_access_randomness(0x6c),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn authored_api_key_scope_and_expiry_lines_are_closed() {
        assert_eq!(parse_api_key_scope_line("").unwrap(), Vec::<String>::new());
        assert_eq!(
            parse_api_key_scope_line("repo,read:org").unwrap(),
            vec!["repo".to_owned(), "read:org".to_owned()]
        );
        // Order is preserved exactly as typed, never sorted or deduplicated.
        assert_eq!(
            parse_api_key_scope_line("write,read").unwrap(),
            vec!["write".to_owned(), "read".to_owned()]
        );
        for rejected in [
            "repo,repo",                                // duplicate component
            "repo, read",                               // untrimmed component
            "repo ",                                    // untrimmed single component
            "repo,",                                    // empty trailing component
            ",repo",                                    // empty leading component
            &"a".repeat(MAX_API_KEY_SCOPE_BYTES + 1),   // oversized component
            &"a,".repeat(MAX_API_KEY_SCOPE_LINE_BYTES), // oversized line
            &(0..=MAX_API_KEY_SCOPES)
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(","), // too many components
        ] {
            assert!(
                matches!(
                    parse_api_key_scope_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "scope line must be rejected"
            );
        }

        assert_eq!(parse_api_key_expiry_line("").unwrap(), None);
        assert_eq!(parse_api_key_expiry_line("1").unwrap(), Some(1));
        assert_eq!(
            parse_api_key_expiry_line("1900000000").unwrap(),
            Some(1_900_000_000)
        );
        for rejected in [
            "0",                    // zero is not an instant
            "01",                   // leading zero is not canonical
            "+1",                   // signs are not accepted
            "-1",                   // signs are not accepted
            "1 ",                   // trailing space is not a digit
            "1e9",                  // no exponent form
            "18446744073709551616", // one past u64::MAX
        ] {
            assert!(
                matches!(
                    parse_api_key_expiry_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "expiry line must be rejected"
            );
        }
    }

    #[test]
    fn audited_authored_api_key_merge_records_host_validation_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x26; 16]);
        let (publication, revisions) = pending_api_key_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(507, audited_access_randomness(0x6d), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_api_key_conflict_merge(
            item_id,
            base_revision,
            508,
            audited_access_randomness(0x6e),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        session
            .prepare_audited_api_key_conflict_merge(
                item_id,
                base_revision,
                509,
                audited_access_randomness(0x6f),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                ApiKeyConflictMergeInputV1::new(
                    Zeroizing::new("Authored key".to_owned()),
                    Zeroizing::new("github.com".to_owned()),
                    Zeroizing::new("authored-token-value".to_owned()),
                    Zeroizing::new("repo,repo".to_owned()),
                    Zeroizing::new("1900000001".to_owned()),
                ),
                resolve_item_conflict_randomness(0x70),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("invalid API-key form must remain a closed failed operation");
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_api_key_conflict_merge(
                item_id,
                base_revision,
                510,
                audited_access_randomness(0x71),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                ApiKeyConflictMergeInputV1::new(
                    Zeroizing::new("Authored key".to_owned()),
                    Zeroizing::new("api.example".to_owned()),
                    Zeroizing::new("authored-token-value".to_owned()),
                    Zeroizing::new("repo,read:org".to_owned()),
                    Zeroizing::new("1900000001".to_owned()),
                ),
                resolve_item_conflict_randomness(0x72),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored API-key merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored API-key merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 500);
        assert_eq!(document.updated_at_ms(), 510);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::ApiKey(api_key) = document.payload() else {
            panic!("authored API-key merge must retain its schema")
        };
        assert_eq!(api_key.label, "Authored key");
        assert_eq!(api_key.service, "api.example");
        assert_eq!(api_key.token, "authored-token-value");
        assert_eq!(
            api_key.scopes,
            vec!["repo".to_owned(), "read:org".to_owned()]
        );
        assert_eq!(api_key.expires_at, Some(1_900_000_001));
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_api_key_merge_accepts_an_empty_scope_and_expiry_form() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x27; 16]);
        let (publication, revisions) = pending_api_key_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(511, audited_access_randomness(0x73), &local)
        .unwrap();

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_api_key_conflict_merge(
            item_id,
            revisions[1],
            512,
            audited_access_randomness(0x74),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .complete_audited(
            ApiKeyConflictMergeInputV1::new(
                Zeroizing::new("Authored key".to_owned()),
                Zeroizing::new("api.example".to_owned()),
                Zeroizing::new("authored-token-value".to_owned()),
                Zeroizing::new(String::new()),
                Zeroizing::new(String::new()),
            ),
            resolve_item_conflict_randomness(0x75),
            &local,
        )
        .unwrap()
        .into_operation()
        .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored API-key merge must be the sole current candidate")
        };
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored API-key merge must publish a live document")
        };
        let AnyRecord::ApiKey(api_key) = document.payload() else {
            panic!("authored API-key merge must retain its schema")
        };
        assert!(api_key.scopes.is_empty());
        assert_eq!(api_key.expires_at, None);
    }

    #[test]
    fn audited_authored_api_key_merge_rejects_a_card_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x28; 16]);
        let (publication, revisions) = pending_card_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(513, audited_access_randomness(0x76), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_api_key_conflict_merge(
            item_id,
            revisions[0],
            514,
            audited_access_randomness(0x77),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_api_key_merge_requires_an_exact_current_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x29; 16]);
        let (publication, _) = pending_api_key_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(515, audited_access_randomness(0x78), &local)
        .unwrap();

        // A revision that is not a current candidate of this item is refused,
        // and the refusal is durable before the caller ever sees it.
        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_api_key_conflict_merge(
            item_id,
            RevisionId::new([0x7a; 32]),
            516,
            audited_access_randomness(0x79),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(result.is_err());
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn authored_database_credential_engine_and_port_lines_are_closed() {
        for accepted in ["postgres", "mysql8", "sql-server", "my_engine", "p"] {
            assert_eq!(validate_database_engine(accepted), Ok(()));
        }
        for rejected in [
            "",                                         // an engine is required
            "Postgres",                                 // uppercase is not canonical
            "9postgres",                                // must start with a letter
            "-postgres",                                // must start with a letter
            "postgres ",                                // untrimmed is not canonical
            "post gres",                                // no interior spaces
            "postgres.db",                              // closed alphabet excludes `.`
            "postgres/db",                              // closed alphabet excludes `/`
            &"p".repeat(MAX_DATABASE_ENGINE_BYTES + 1), // oversized identifier
            "pö",                                       // ASCII only
        ] {
            assert!(
                matches!(
                    validate_database_engine(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "engine must be rejected"
            );
        }

        assert_eq!(parse_database_port_line("1"), Ok(1));
        assert_eq!(parse_database_port_line("5432"), Ok(5432));
        assert_eq!(parse_database_port_line("65535"), Ok(65535));
        for rejected in [
            "",       // a port is required
            "0",      // zero is not a TCP port
            "05432",  // leading zero is not canonical
            "+5432",  // signs are not accepted
            "-5432",  // signs are not accepted
            "5432 ",  // trailing space is not a digit
            "65536",  // one past the u16 port space
            "0x1538", // no alternate radix
        ] {
            assert!(
                matches!(
                    parse_database_port_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "port line must be rejected"
            );
        }
    }

    #[test]
    fn audited_authored_database_credential_merge_records_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2e; 16]);
        let (publication, revisions) =
            pending_database_credential_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(517, audited_access_randomness(0x7b), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_database_credential_conflict_merge(
            item_id,
            base_revision,
            518,
            audited_access_randomness(0x7c),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        // An invalid port is caught behind the audited boundary, so the closed
        // failure is already durable when the caller learns about it.
        session
            .prepare_audited_database_credential_conflict_merge(
                item_id,
                base_revision,
                519,
                audited_access_randomness(0x7d),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                DatabaseCredentialConflictMergeInputV1::new(
                    Zeroizing::new("Authored database".to_owned()),
                    Zeroizing::new("postgres".to_owned()),
                    Zeroizing::new("db.example".to_owned()),
                    Zeroizing::new("65536".to_owned()),
                    Some(Zeroizing::new("orders".to_owned())),
                    Zeroizing::new("service".to_owned()),
                    Zeroizing::new("authored-password-value".to_owned()),
                ),
                resolve_item_conflict_randomness(0x7e),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("invalid database-credential form must remain a closed failed operation");
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        // An invalid engine is refused by the same boundary.
        session
            .prepare_audited_database_credential_conflict_merge(
                item_id,
                base_revision,
                520,
                audited_access_randomness(0x7f),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                DatabaseCredentialConflictMergeInputV1::new(
                    Zeroizing::new("Authored database".to_owned()),
                    Zeroizing::new("Postgres".to_owned()),
                    Zeroizing::new("db.example".to_owned()),
                    Zeroizing::new("5432".to_owned()),
                    Some(Zeroizing::new("orders".to_owned())),
                    Zeroizing::new("service".to_owned()),
                    Zeroizing::new("authored-password-value".to_owned()),
                ),
                resolve_item_conflict_randomness(0x80),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("invalid database engine must remain a closed failed operation");
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_database_credential_conflict_merge(
                item_id,
                base_revision,
                521,
                audited_access_randomness(0x81),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                DatabaseCredentialConflictMergeInputV1::new(
                    Zeroizing::new("Authored database".to_owned()),
                    Zeroizing::new("mysql".to_owned()),
                    Zeroizing::new("replica.example".to_owned()),
                    Zeroizing::new("3306".to_owned()),
                    Some(Zeroizing::new("ledger".to_owned())),
                    Zeroizing::new("merged-service".to_owned()),
                    Zeroizing::new("authored-password-value".to_owned()),
                ),
                resolve_item_conflict_randomness(0x82),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored database-credential merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored database-credential merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 500);
        assert_eq!(document.updated_at_ms(), 521);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::DatabaseCredential(credential) = document.payload() else {
            panic!("authored database-credential merge must retain its schema")
        };
        assert_eq!(credential.label, "Authored database");
        assert_eq!(credential.engine, "mysql");
        assert_eq!(credential.host, "replica.example");
        assert_eq!(credential.port, 3306);
        assert_eq!(credential.database.as_deref(), Some("ledger"));
        assert_eq!(credential.username, "merged-service");
        assert_eq!(credential.password, "authored-password-value");
        // An authored credential is static: VLT-PM37 §1 keeps the lease absent
        // rather than inheriting one from the base candidate.
        assert_eq!(credential.lease_id, None);
        assert_eq!(credential.expires_at, None);
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_database_credential_merge_accepts_an_absent_database_name() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2f; 16]);
        let (publication, revisions) =
            pending_database_credential_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(522, audited_access_randomness(0x83), &local)
        .unwrap();

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_database_credential_conflict_merge(
            item_id,
            revisions[1],
            523,
            audited_access_randomness(0x84),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .complete_audited(
            DatabaseCredentialConflictMergeInputV1::new(
                Zeroizing::new("Authored database".to_owned()),
                Zeroizing::new("postgres".to_owned()),
                Zeroizing::new("db.example".to_owned()),
                Zeroizing::new("5432".to_owned()),
                None,
                Zeroizing::new("service".to_owned()),
                Zeroizing::new("authored-password-value".to_owned()),
            ),
            resolve_item_conflict_randomness(0x85),
            &local,
        )
        .unwrap()
        .into_operation()
        .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored database-credential merge must be the sole current candidate")
        };
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored database-credential merge must publish a live document")
        };
        let AnyRecord::DatabaseCredential(credential) = document.payload() else {
            panic!("authored database-credential merge must retain its schema")
        };
        assert_eq!(credential.database, None);
        assert_eq!(credential.lease_id, None);
        assert_eq!(credential.expires_at, None);
    }

    #[test]
    fn audited_authored_database_credential_merge_rejects_an_api_key_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x30; 16]);
        let (publication, revisions) = pending_api_key_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(524, audited_access_randomness(0x86), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_database_credential_conflict_merge(
            item_id,
            revisions[0],
            525,
            audited_access_randomness(0x87),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_database_credential_merge_requires_an_exact_current_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x31; 16]);
        let (publication, _) = pending_database_credential_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(526, audited_access_randomness(0x88), &local)
        .unwrap();

        // A revision that is not a current candidate of this item is refused,
        // and the refusal is durable before the caller ever sees it.
        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_database_credential_conflict_merge(
            item_id,
            RevisionId::new([0x89; 32]),
            527,
            audited_access_randomness(0x8a),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(result.is_err());
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn authored_totp_secret_and_parameter_lines_are_closed() {
        for accepted in ["SHA1", "SHA256", "SHA512"] {
            assert_eq!(validate_totp_algorithm(accepted), Ok(()));
        }
        for rejected in [
            "",         // an algorithm is required
            "sha1",     // lowercase is not canonical
            "SHA-1",    // the closed set has no punctuation spelling
            "SHA1 ",    // untrimmed is not canonical
            "SHA224",   // not a TOTP construction
            "MD5",      // not a TOTP construction
            "SHA1SHA1", // no repetition
        ] {
            assert!(
                matches!(
                    validate_totp_algorithm(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "algorithm must be rejected"
            );
        }

        assert_eq!(parse_totp_digits_line("6"), Ok(6));
        assert_eq!(parse_totp_digits_line("8"), Ok(8));
        for rejected in ["", "4", "5", "7", "9", "10", "06", "6 ", "+6", "six"] {
            assert!(
                matches!(
                    parse_totp_digits_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "digit count must be rejected"
            );
        }

        assert_eq!(parse_totp_period_line("1"), Ok(1));
        assert_eq!(parse_totp_period_line("30"), Ok(30));
        assert_eq!(parse_totp_period_line("3600"), Ok(3_600));
        for rejected in [
            "",      // a period is required
            "0",     // zero is not a time step
            "030",   // leading zero is not canonical
            "+30",   // signs are not accepted
            "-30",   // signs are not accepted
            "30 ",   // trailing space is not a digit
            "3601",  // one past the accepted ceiling
            "0x1e",  // no alternate radix
            "99999", // far past the ceiling
        ] {
            assert!(
                matches!(
                    parse_totp_period_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "period line must be rejected"
            );
        }

        // Canonical Base32 round trips: each vector is the only spelling of
        // its bytes, so decoding and re-encoding must return the input.
        for (encoded, bytes) in [
            ("AA", [0_u8].as_slice()),
            ("7A", [0xf8].as_slice()),
            ("JBSWY3DP", b"Hello".as_slice()),
            ("JBSWY3DPEHPK3PXP", b"Hello!\xde\xad\xbe\xef".as_slice()),
        ] {
            let decoded = decode_totp_secret_line(encoded).unwrap();
            assert_eq!(decoded.as_slice(), bytes);
            assert_eq!(encode_totp_secret_base32(&decoded).as_str(), encoded);
        }
        for rejected in [
            "",                                            // a seed is required
            "A",                                           // five bits decode to no byte
            "jbswy3dp",                                    // lowercase is not canonical
            "JBSWY3DP=",                                   // unpadded Base32 only
            "JBSWY3D0",                                    // `0` is not in the alphabet
            "JBSWY3D1",                                    // `1` is not in the alphabet
            "JBSWY3D-",                                    // punctuation is not in the alphabet
            "JBSWY3DÖ",                                    // ASCII only
            "AB",                                          // 3 unused trailing bits are nonzero
            &"A".repeat(MAX_TOTP_SECRET_BASE32_CHARS + 1), // oversized line
        ] {
            assert!(
                matches!(
                    decode_totp_secret_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "seed line must be rejected: {rejected}"
            );
        }
        // The longest accepted line carries exactly the 160-byte ceiling
        // VLT-PM29 places on a stored seed.
        assert_eq!(
            decode_totp_secret_line(&"A".repeat(MAX_TOTP_SECRET_BASE32_CHARS))
                .unwrap()
                .len(),
            160
        );
    }

    #[test]
    fn audited_authored_totp_merge_records_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x32; 16]);
        let (publication, revisions) = pending_totp_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(528, audited_access_randomness(0x8b), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_totp_conflict_merge(
            item_id,
            base_revision,
            529,
            audited_access_randomness(0x8c),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        // A noncanonical seed line is caught behind the audited boundary, so
        // the closed failure is already durable when the caller learns of it.
        session
            .prepare_audited_totp_conflict_merge(
                item_id,
                base_revision,
                530,
                audited_access_randomness(0x8d),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                TotpConflictMergeInputV1::new(
                    Zeroizing::new("Authored seed".to_owned()),
                    Some(Zeroizing::new("Example".to_owned())),
                    Zeroizing::new("jbswy3dp".to_owned()),
                    Zeroizing::new("SHA1".to_owned()),
                    Zeroizing::new("6".to_owned()),
                    Zeroizing::new("30".to_owned()),
                ),
                resolve_item_conflict_randomness(0x8e),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("noncanonical TOTP seed must remain a closed failed operation");
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        // An out-of-set algorithm is refused by the same boundary.
        session
            .prepare_audited_totp_conflict_merge(
                item_id,
                base_revision,
                531,
                audited_access_randomness(0x8f),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                TotpConflictMergeInputV1::new(
                    Zeroizing::new("Authored seed".to_owned()),
                    Some(Zeroizing::new("Example".to_owned())),
                    Zeroizing::new("JBSWY3DP".to_owned()),
                    Zeroizing::new("SHA224".to_owned()),
                    Zeroizing::new("6".to_owned()),
                    Zeroizing::new("30".to_owned()),
                ),
                resolve_item_conflict_randomness(0x90),
                &local,
            )
            .unwrap()
            .into_operation()
            .expect_err("unknown TOTP algorithm must remain a closed failed operation");
        // So are an out-of-set digit count and an out-of-range period.
        for (digits, period, seed) in [("7", "30", 0x91), ("6", "3601", 0x93)] {
            let session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(session.conflicted_item_count(), 1);
            session
                .prepare_audited_totp_conflict_merge(
                    item_id,
                    base_revision,
                    532,
                    audited_access_randomness(seed),
                    &local,
                )
                .unwrap()
                .into_preparation()
                .unwrap()
                .complete_audited(
                    TotpConflictMergeInputV1::new(
                        Zeroizing::new("Authored seed".to_owned()),
                        None,
                        Zeroizing::new("JBSWY3DP".to_owned()),
                        Zeroizing::new("SHA1".to_owned()),
                        Zeroizing::new(digits.to_owned()),
                        Zeroizing::new(period.to_owned()),
                    ),
                    resolve_item_conflict_randomness(seed + 1),
                    &local,
                )
                .unwrap()
                .into_operation()
                .expect_err("out-of-set TOTP parameter must remain a closed failed operation");
        }
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();

        session
            .prepare_audited_totp_conflict_merge(
                item_id,
                base_revision,
                533,
                audited_access_randomness(0x95),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                TotpConflictMergeInputV1::new(
                    Zeroizing::new("Authored seed".to_owned()),
                    Some(Zeroizing::new("Merged issuer".to_owned())),
                    Zeroizing::new("JBSWY3DPEHPK3PXP".to_owned()),
                    Zeroizing::new("SHA256".to_owned()),
                    Zeroizing::new("8".to_owned()),
                    Zeroizing::new("60".to_owned()),
                ),
                resolve_item_conflict_randomness(0x96),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored TOTP merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored TOTP merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 500);
        assert_eq!(document.updated_at_ms(), 533);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        let AnyRecord::TotpSeed(seed) = document.payload() else {
            panic!("authored TOTP merge must retain its schema")
        };
        assert_eq!(seed.label, "Authored seed");
        assert_eq!(seed.issuer.as_deref(), Some("Merged issuer"));
        assert_eq!(seed.secret, b"Hello!\xde\xad\xbe\xef");
        assert_eq!(seed.algorithm, "SHA256");
        assert_eq!(seed.digits, 8);
        assert_eq!(seed.period, 60);
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_totp_merge_accepts_an_absent_issuer() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x33; 16]);
        let (publication, revisions) = pending_totp_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(534, audited_access_randomness(0x97), &local)
        .unwrap();

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_totp_conflict_merge(
            item_id,
            revisions[1],
            535,
            audited_access_randomness(0x98),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .complete_audited(
            TotpConflictMergeInputV1::new(
                Zeroizing::new("Authored seed".to_owned()),
                None,
                Zeroizing::new("JBSWY3DP".to_owned()),
                Zeroizing::new("SHA1".to_owned()),
                Zeroizing::new("6".to_owned()),
                Zeroizing::new("30".to_owned()),
            ),
            resolve_item_conflict_randomness(0x99),
            &local,
        )
        .unwrap()
        .into_operation()
        .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored TOTP merge must be the sole current candidate")
        };
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored TOTP merge must publish a live document")
        };
        let AnyRecord::TotpSeed(seed) = document.payload() else {
            panic!("authored TOTP merge must retain its schema")
        };
        // An absent issuer stays absent rather than being inherited from the
        // base candidate, which carried one.
        assert_eq!(seed.issuer, None);
        assert_eq!(seed.secret, b"Hello");
    }

    #[test]
    fn audited_authored_totp_merge_rejects_a_database_credential_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x34; 16]);
        let (publication, revisions) =
            pending_database_credential_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(536, audited_access_randomness(0x9a), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_totp_conflict_merge(
            item_id,
            revisions[0],
            537,
            audited_access_randomness(0x9b),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_totp_merge_requires_an_exact_current_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x35; 16]);
        let (publication, _) = pending_totp_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(538, audited_access_randomness(0x9c), &local)
        .unwrap();

        // A revision that is not a current candidate of this item is refused,
        // and the refusal is durable before the caller ever sees it.
        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_totp_conflict_merge(
            item_id,
            RevisionId::new([0x9d; 32]),
            539,
            audited_access_randomness(0x9e),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(result.is_err());
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn authored_opaque_payload_lines_are_closed() {
        // The hexadecimal layer: two lowercase characters spell one byte, and
        // nothing else is a spelling of anything.
        for (line, bytes) in [
            ("00", [0x00_u8].as_slice()),
            ("a0", [0xa0].as_slice()),
            ("ff", [0xff].as_slice()),
            (
                "a2616b016176666d6572676564",
                [
                    0xa2, 0x61, b'k', 0x01, 0x61, b'v', 0x66, b'm', b'e', b'r', b'g', b'e', b'd',
                ]
                .as_slice(),
            ),
        ] {
            assert_eq!(decode_opaque_payload_line(line).unwrap().as_slice(), bytes);
        }
        for rejected in [
            "",                                            // a payload is required
            "a",                                           // a lone nibble spells no byte
            "a0b",   // odd length, so the last byte is incomplete
            "A0",    // uppercase is a second spelling of the same byte
            "a0zz",  // not hexadecimal at all
            "a0 00", // odd length first, and a space is not a hex digit either
            "0xa0",  // no radix prefix
            &"0".repeat(MAX_OPAQUE_PAYLOAD_HEX_CHARS + 2), // oversized line
        ] {
            assert!(
                matches!(
                    decode_opaque_payload_line(rejected),
                    Err(ApplicationError::InvalidInput)
                ),
                "payload line must be rejected: {rejected}"
            );
        }
        // The longest accepted line carries exactly the 512-byte payload
        // ceiling that the host's 1,024-byte secret line can spell.
        assert_eq!(
            decode_opaque_payload_line(&"0".repeat(MAX_OPAQUE_PAYLOAD_HEX_CHARS))
                .unwrap()
                .len(),
            512
        );

        // The CBOR layer: an accepted payload comes back from the envelope
        // round trip byte for byte.
        for accepted in ["00", "a0", "a2616b016176666d6572676564"] {
            let typed = decode_opaque_payload_line(accepted).unwrap();
            assert_eq!(
                canonical_opaque_payload(FIXTURE_OPAQUE_CONTENT_TYPE, &typed)
                    .unwrap()
                    .as_slice(),
                typed.as_slice()
            );
        }
        for rejected in [
            "ff",             // simple value 31 is not a canonical CBOR value
            "a0a0",           // one value per payload; the second is trailing
            "a2617601616b01", // map keys "v" then "k" are out of canonical order
            "1800",           // a one-byte argument for 0 is not the minimal form
            "a16176",         // a map header with no value for its key
        ] {
            let typed = decode_opaque_payload_line(rejected).unwrap();
            assert!(
                matches!(
                    canonical_opaque_payload(FIXTURE_OPAQUE_CONTENT_TYPE, &typed),
                    Err(ApplicationError::InvalidInput)
                ),
                "payload must be rejected: {rejected}"
            );
        }
        // A payload nested exactly as deep as the decoder allows is one level
        // too deep once the content-type envelope is wrapped around it. The
        // ceremony refuses it rather than letting the encoder fail.
        let too_deep = decode_opaque_payload_line(&format!(
            "{}00",
            "81".repeat(coding_adventures_canonical_cbor::MAX_DECODE_DEPTH)
        ))
        .unwrap();
        assert!(matches!(
            canonical_opaque_payload(FIXTURE_OPAQUE_CONTENT_TYPE, &too_deep),
            Err(ApplicationError::InvalidInput)
        ));

        // A payload that round trips as a first-party record cannot reach here
        // from a base that decoded as opaque, and is refused as an invariant
        // failure if it ever does, so this command can never author a login or
        // a note without their own closed field rules.
        let secure_note_payload = [
            0xa2, 0x64, b'b', b'o', b'd', b'y', 0x61, b'b', 0x65, b't', b'i', b't', b'l', b'e',
            0x61, b't',
        ];
        assert!(matches!(
            canonical_opaque_payload(
                coding_adventures_vault_records::SECURE_NOTE_V1,
                &secure_note_payload,
            ),
            Err(ApplicationError::InternalInvariant)
        ));
    }

    #[test]
    fn audited_authored_opaque_merge_records_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x36; 16]);
        let (publication, revisions) = pending_opaque_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(540, audited_access_randomness(0xa0), &local)
        .unwrap();

        let base_revision = revisions[0];
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_opaque_conflict_merge(
            item_id,
            base_revision,
            541,
            audited_access_randomness(0xa1),
            &local,
        )
        .unwrap()
        .into_preparation()
        .unwrap()
        .record_audited_host_failure(&local)
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        // An uppercase line and a non-canonical CBOR map are both caught behind
        // the audited boundary, so each closed failure is already durable when
        // the caller learns of it.
        for (line, seed) in [("A0", 0xa2_u8), ("a2617601616b01", 0xa4)] {
            let session = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(session.conflicted_item_count(), 1);
            session
                .prepare_audited_opaque_conflict_merge(
                    item_id,
                    base_revision,
                    542,
                    audited_access_randomness(seed),
                    &local,
                )
                .unwrap()
                .into_preparation()
                .unwrap()
                .complete_audited(
                    OpaqueConflictMergeInputV1::new(Zeroizing::new(line.to_owned())),
                    resolve_item_conflict_randomness(seed + 1),
                    &local,
                )
                .unwrap()
                .into_operation()
                .expect_err("an invalid opaque payload must remain a closed failed operation");
            let reopened = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(
                latest_audit_facts(&reopened),
                (
                    AuditActionV1::ItemConflictMerge,
                    AuditOutcomeV1::Failed,
                    Some(item_id),
                    None,
                )
            );
        }

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.conflicted_item_count(), 1);
        let base_document = session.current_catalog.items[&item_id]
            .iter()
            .find(|candidate| candidate.revision_id() == base_revision)
            .and_then(|candidate| match candidate.state() {
                ItemState::Live(document) => Some(document),
                ItemState::Tombstone(_) => None,
            })
            .unwrap();
        let expected_favorite = base_document.favorite().clone();
        let expected_collections = base_document.collection_ids().clone();
        let expected_tags = base_document.tags().clone();
        let expected_attachments = base_document.attachments().clone();
        let expected_schema = base_document.schema().clone();

        session
            .prepare_audited_opaque_conflict_merge(
                item_id,
                base_revision,
                543,
                audited_access_randomness(0xa6),
                &local,
            )
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                OpaqueConflictMergeInputV1::new(Zeroizing::new(
                    "a2616b016176666d6572676564".to_owned(),
                )),
                resolve_item_conflict_randomness(0xa7),
                &local,
            )
            .unwrap()
            .into_operation()
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 0);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                None,
            )
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("authored opaque merge must be the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &revisions.iter().copied().collect::<BTreeSet<_>>()
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("authored opaque merge must publish a live document")
        };
        assert_eq!(document.created_at_ms(), 500);
        assert_eq!(document.updated_at_ms(), 543);
        assert_eq!(document.favorite(), &expected_favorite);
        assert_eq!(document.collection_ids(), &expected_collections);
        assert_eq!(document.tags(), &expected_tags);
        assert_eq!(document.attachments(), &expected_attachments);
        assert_eq!(document.schema(), &expected_schema);
        let AnyRecord::Opaque {
            content_type,
            payload_bytes,
        } = document.payload()
        else {
            panic!("authored opaque merge must retain its schema")
        };
        // The content type is inherited from the base rather than authored,
        // and the payload is exactly the bytes the line spelled.
        assert_eq!(content_type, FIXTURE_OPAQUE_CONTENT_TYPE);
        assert_eq!(
            payload_bytes.as_slice(),
            [0xa2, 0x61, b'k', 0x01, 0x61, b'v', 0x66, b'm', b'e', b'r', b'g', b'e', b'd']
        );
        assert_eq!(reopened.item_history(item_id, 100).unwrap().len(), 3);
    }

    #[test]
    fn audited_authored_opaque_merge_rejects_a_first_party_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x37; 16]);
        let (publication, revisions) = pending_secure_note_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(544, audited_access_randomness(0xa8), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_opaque_conflict_merge(
            item_id,
            revisions[0],
            545,
            audited_access_randomness(0xa9),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_opaque_merge_requires_an_exact_current_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x38; 16]);
        let (publication, _) = pending_opaque_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(546, audited_access_randomness(0xaa), &local)
        .unwrap();

        // A revision that is not a current candidate of this item is refused,
        // and the refusal is durable before the caller ever sees it.
        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_opaque_conflict_merge(
            item_id,
            RevisionId::new([0xab; 32]),
            547,
            audited_access_randomness(0xac),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(result.is_err());
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn audited_authored_secure_note_merge_rejects_a_login_base() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x21; 16]);
        let (publication, revisions) = pending_live_conflict_publication(&active, item_id);
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::pending_publication(active, publication)
                .unwrap()
                .encode()
                .unwrap(),
        );
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(416, audited_access_randomness(0x63), &local)
        .unwrap();

        let result = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_secure_note_conflict_merge(
            item_id,
            revisions[0].0,
            417,
            audited_access_randomness(0x64),
            &local,
        )
        .unwrap()
        .into_preparation();
        assert!(matches!(result, Err(ApplicationError::Unsupported)));
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.conflicted_item_count(), 1);
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::ItemConflictMerge,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
    }

    #[test]
    fn authored_conflict_merge_rejects_missing_sole_and_changed_identity_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let missing = ItemId::new([0x26; 16]);
        let exact_empty = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                new_login_document(missing, "Missing", "missing-secret"),
                406,
                resolve_item_conflict_randomness(0x52),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_empty).unwrap()
        else {
            panic!("fixture must be active")
        };
        let sole_id = ItemId::new([0x27; 16]);
        let publication = pending_live_publication(&active, sole_id, "Sole", "sole-secret");
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_sole = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                login_document_with_times(sole_id, "Not a conflict", "secret", 100, 300),
                407,
                resolve_item_conflict_randomness(0x53),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_sole.as_slice())
        );

        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_sole).unwrap()
        else {
            panic!("fixture must be active")
        };
        let conflict_id = ItemId::new([0x28; 16]);
        let (publication, _) = pending_live_conflict_publication(&active, conflict_id);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_conflict = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                login_document_with_times(conflict_id, "Changed identity", "secret", 301, 301,),
                408,
                resolve_item_conflict_randomness(0x54),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_conflict.as_slice())
        );
    }

    #[test]
    fn authored_conflict_merge_rejects_all_tombstone_conflict_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x29; 16]);
        let (publication, _) = pending_tombstone_publication(&active, item_id, item_id, 2, None);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_conflict = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .merge_item_conflict(
                new_login_document(item_id, "Cannot revive", "secret"),
                409,
                resolve_item_conflict_randomness(0x55),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_conflict.as_slice())
        );
    }

    #[test]
    fn conflict_resolution_rejects_missing_or_unconflicted_revisions_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            session.conflict_candidates(ItemId::new([0x2b; 16])),
            Err(ApplicationError::NotFound)
        );
        let exact_empty = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.resolve_item_conflict(
                RevisionId::new([0x2c; 32]),
                410,
                resolve_item_conflict_randomness(0x5d),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        let add_randomness = add_item_randomness(0x6d);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Sole candidate", "only-secret"),
            411,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            session.conflict_candidates(item_id),
            Err(ApplicationError::ConflictRequired)
        );
        let sole_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_sole = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.resolve_item_conflict(
                sole_revision,
                412,
                resolve_item_conflict_randomness(0x7d),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_sole.as_slice())
        );
    }

    #[test]
    fn conflict_resolution_can_choose_a_retained_tombstone_without_losing_parents() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x2d; 16]);
        let (publication, revisions) =
            pending_tombstone_publication(&active, item_id, item_id, 2, None);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let views = session.conflict_candidates(item_id).unwrap();
        assert!(views.iter().all(ItemHistoryViewV1::is_deleted));
        session
            .resolve_item_conflict(
                revisions[1],
                420,
                resolve_item_conflict_randomness(0x8d),
                &local,
            )
            .unwrap();

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.get_item(item_id).unwrap(), None);
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("resolution must become the sole current candidate")
        };
        assert!(matches!(candidate.state(), ItemState::Tombstone(_)));
        assert_eq!(
            candidate.causal_parents(),
            &revisions.into_iter().collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn active_open_rejects_catalog_revision_item_mismatch() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let (publication, _) = pending_tombstone_publication(
            &active,
            ItemId::new([0x21; 16]),
            ItemId::new([0x22; 16]),
            1,
            None,
        );
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_rejects_dangling_catalog_revision() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let publication = pending_dangling_catalog(&active);
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_rejects_dangling_causal_parent() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x41; 16]);
        let (publication, _) = pending_tombstone_publication(
            &active,
            item_id,
            item_id,
            1,
            Some(RevisionId::new([0x42; 32])),
        );
        let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
        *local.0.lock().unwrap() = Some(pending.encode().unwrap());
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_validates_existing_causal_parent_item_binding() {
        for matching_parent in [true, false] {
            let (locator, local, bootstrap, factory) = initialized();
            let exact_active = local.0.lock().unwrap().clone().unwrap();
            let LocalVaultStateV1::Active(active) =
                LocalVaultStateV1::decode(&exact_active).unwrap()
            else {
                panic!("fixture must be active")
            };
            let item_id = ItemId::new([0x51; 16]);
            let parent_item_id = if matching_parent {
                item_id
            } else {
                ItemId::new([0x52; 16])
            };
            let publication = pending_child_publication(&active, item_id, parent_item_id);
            let pending = LocalVaultStateV1::pending_publication(active, publication).unwrap();
            *local.0.lock().unwrap() = Some(pending.encode().unwrap());
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();

            let result = open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            );
            if matching_parent {
                let session = result.unwrap();
                assert_eq!(session.item_count(), 1);
                assert_eq!(session.candidate_count(), 1);
            } else {
                assert_eq!(result.err(), Some(ApplicationError::IntegrityFailure));
            }
        }
    }

    #[test]
    fn active_open_closes_wrong_passphrase_and_bootstrap_rollback() {
        let (locator, local, bootstrap, factory) = initialized();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"wrong".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );

        bootstrap.latest.lock().unwrap().as_mut().unwrap()[0] ^= 1;
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn active_open_requires_matching_locator_and_stable_state() {
        let (locator, local, bootstrap, factory) = initialized();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                BootstrapLocator::new([0x99; 32]),
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );

        *local.0.lock().unwrap() = None;
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::NotInitialized)
        );
    }

    #[test]
    fn active_open_rejects_unfinished_initialization() {
        let prepared = prepare_generation_zero(
            Zeroizing::new(b"active passphrase".to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::with_state(prepared.owner_state().encode().unwrap());
        let bootstrap = MemoryBootstrapStore::default();
        let factory = V1ApplicationRepositoryFactory::new(InMemoryObjectStore::new());

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::InvalidInput)
        );
    }

    #[test]
    fn add_item_publishes_parentless_revision_and_requires_reopen() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        let randomness = add_item_randomness(0x41);
        let item_id = randomness.item_id();
        local.concurrent_winner_on_next_compare();
        let active = session
            .add_item(
                new_login_document(item_id, "New portal", "new-password-secret"),
                301,
                randomness,
                &local,
            )
            .unwrap();

        assert_eq!(active.last_device_counter(), 2);
        assert_ne!(active.pinned_heads(), &prior_heads);
        assert_eq!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(active.clone())
        );

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 1);
        assert_eq!(reopened.search_item_count(), 1);
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("new item must have exactly one current revision")
        };
        assert!(candidate.causal_parents().is_empty());
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(commit.device_counter(), 2);
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.catalog_root(), active.catalog_root());
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 301);
    }

    #[test]
    fn active_audit_item_create_is_signed_encrypted_and_atomic_with_the_commit() {
        let (locator, local, bootstrap, factory) = initialized();
        let mut session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        let previous_event = ObjectId::new([0xee; 32]);
        let exact_prior = LocalVaultStateV1::Active(session.active.clone())
            .encode()
            .unwrap();
        session.active = session
            .active
            .clone()
            .with_audit_event_head(previous_event)
            .unwrap();
        let exact_audited = LocalVaultStateV1::Active(session.active.clone())
            .encode()
            .unwrap();
        local
            .compare_exchange(locator, Some(&exact_prior), &exact_audited)
            .unwrap();
        let (device_public, _) = generate_keypair(session._local_secret.device_signing_seed());
        let randomness = add_item_randomness(0x42);
        let item_id = randomness.item_id();
        let active = session
            .add_item(
                new_login_document(item_id, "Audited portal", "audited-secret"),
                302,
                randomness,
                &local,
            )
            .unwrap();
        let audit_head = active.audit_event_head().unwrap();
        assert_ne!(audit_head, previous_event);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(commit.added_objects().len(), 3);
        assert!(commit.added_objects().contains(&audit_head));
        let audit_object = reopened._repository.read_object(audit_head).unwrap();
        let plaintext = open_object(
            &reopened._keys,
            ObjectKind::AuditEvent,
            audit_object.frame(),
        )
        .unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        event.verify(&device_public).unwrap();
        assert_eq!(event.event().action(), AuditActionV1::ItemCreate);
        assert_eq!(event.event().item_id(), Some(item_id));
        assert_eq!(event.event().selected_revision(), None);
        assert_eq!(event.event().previous_event(), Some(previous_event));
        assert_eq!(
            event.event().basis_heads(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(event.event().device_counter(), 2);
        assert_eq!(event.event().timestamp_ms(), 302);
        assert_eq!(
            event.event().result_revision().unwrap().as_bytes(),
            commit
                .added_objects()
                .iter()
                .find(|id| **id != audit_head && **id != commit.catalog_root())
                .unwrap()
                .as_bytes()
        );
    }

    #[test]
    fn add_item_rejects_mismatched_or_existing_random_identity_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let mismatched = add_item_randomness(0x51);
        assert_eq!(
            session.add_item(
                new_login_document(ItemId::new([0x99; 16]), "Wrong", "secret"),
                301,
                mismatched,
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0x61);
        let item_id = randomness.item_id();
        session
            .add_item(
                new_login_document(item_id, "First", "secret"),
                302,
                randomness,
                &local,
            )
            .unwrap();
        let exact_after_first = local.0.lock().unwrap().clone().unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let mut duplicate_bytes = [0; ADD_ITEM_RANDOM_BYTES];
        duplicate_bytes[..16].copy_from_slice(item_id.as_bytes());
        assert_eq!(
            session.add_item(
                new_login_document(item_id, "Duplicate", "secret"),
                303,
                AddItemRandomnessV1::new(duplicate_bytes),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_after_first.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let second_randomness = add_item_randomness(0x71);
        let second_item_id = second_randomness.item_id();
        session
            .add_item(
                new_login_document(second_item_id, "Second", "secret"),
                304,
                second_randomness,
                &local,
            )
            .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 2);
        assert!(reopened.get_item(item_id).unwrap().is_some());
        assert!(reopened.get_item(second_item_id).unwrap().is_some());
    }

    #[test]
    fn add_item_retains_exact_pending_journal_across_ambiguous_provider_failure() {
        let passphrase = b"active passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        let session = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let randomness = add_item_randomness(0x81);
        let item_id = randomness.item_id();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert_eq!(
            session.add_item(
                new_login_document(item_id, "Crash safe", "secret"),
                304,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&exact_pending).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));

        let active = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.last_device_counter(), 2);
        let reopened = open_active_vault(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn add_item_persists_before_publish_and_recovers_after_final_cas_failure() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let randomness = add_item_randomness(0x91);
        let item_id = randomness.item_id();
        local.fail_next_compare(LocalStateStoreError::Unavailable);
        assert_eq!(
            session.add_item(
                new_login_document(item_id, "No early publish", "secret"),
                305,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.open_report().commit_count(), 1);

        let randomness = add_item_randomness(0xa1);
        let item_id = randomness.item_id();
        local.fail_compare_after(1, LocalStateStoreError::Unavailable);
        assert_eq!(
            reopened.add_item(
                new_login_document(item_id, "Recover final state", "secret"),
                306,
                randomness,
                &local,
            ),
            Err(ApplicationError::StorageUnavailable)
        );
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        assert!(matches!(
            LocalVaultStateV1::decode(&exact_pending).unwrap(),
            LocalVaultStateV1::PendingPublication { .. }
        ));
        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            reopened.get_item(item_id).unwrap().unwrap().item_id,
            item_id
        );
        assert_eq!(reopened.open_report().commit_count(), 2);
    }

    #[test]
    fn replace_item_advances_one_expected_live_revision_and_preserves_others() {
        let (locator, local, bootstrap, factory) = initialized();
        let first_randomness = add_item_randomness(0xb1);
        let first_item_id = first_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(first_item_id, "Before", "old-secret"),
            401,
            first_randomness,
            &local,
        )
        .unwrap();
        let second_randomness = add_item_randomness(0xc1);
        let second_item_id = second_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(second_item_id, "Untouched", "other-secret"),
            402,
            second_randomness,
            &local,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let prior_heads = session.local_pins().clone();
        let expected_revision = session.current_catalog.items[&first_item_id][0].revision_id();
        let active = session
            .replace_item(
                expected_revision,
                new_login_document(first_item_id, "After", "new-secret"),
                403,
                replace_item_randomness(0xd1),
                &local,
            )
            .unwrap();

        assert_eq!(active.last_device_counter(), 4);
        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 2);
        assert!(reopened.get_item(second_item_id).unwrap().is_some());
        let [candidate] = reopened.current_catalog.items[&first_item_id].as_slice() else {
            panic!("replacement must become the sole current candidate")
        };
        assert_ne!(candidate.revision_id(), expected_revision);
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([expected_revision])
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("replacement must remain live")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("replacement schema must remain login")
        };
        assert_eq!(login.title, "After");
        assert_eq!(login.password, "new-secret");
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 403);
    }

    #[test]
    fn replace_item_rejects_missing_stale_and_immutable_identity_changes_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let missing_randomness = add_item_randomness(0xe1);
        let item_id = missing_randomness.item_id();
        let exact_empty = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .replace_item(
                RevisionId::new([0x91; 32]),
                new_login_document(item_id, "Missing", "secret"),
                404,
                replace_item_randomness(0xf1),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_empty.as_slice())
        );

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Original", "secret"),
            405,
            missing_randomness,
            &local,
        )
        .unwrap();
        let exact_item = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .replace_item(
                RevisionId::new([0x92; 32]),
                new_login_document(item_id, "Stale", "secret"),
                406,
                replace_item_randomness(0x01),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_item.as_slice())
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
        let changed_creation = ItemDocument::new(
            item_id,
            ContentType::new(LOGIN_V1).unwrap(),
            299,
            300,
            LwwRegister::new(false, 300, OperationId::new([0x72; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Login(Login {
                title: "Changed creation".to_owned(),
                username: "new-user@example.test".to_owned(),
                password: "secret".to_owned(),
                urls: vec!["https://new.example.test".to_owned()],
                notes: None,
            }),
            ObservedSet::new(),
        )
        .unwrap();
        assert_eq!(
            session.replace_item(
                expected_revision,
                changed_creation,
                407,
                replace_item_randomness(0x11),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_item.as_slice())
        );
    }

    #[test]
    fn audited_item_create_host_failure_is_durable_and_traceable() {
        let (locator, local, bootstrap, factory) = initialized();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(430, audited_access_randomness(0x61), &local)
        .unwrap();

        let add_randomness = add_item_randomness(0x62);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .record_audited_item_create_host_failure(
            add_randomness,
            431,
            audited_access_randomness(0x63),
            &local,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemCreate,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let event_head = session.active.audit_event_head().unwrap();
        let object = session._repository.read_object(event_head).unwrap();
        let plaintext =
            open_object(&session._keys, ObjectKind::AuditEvent, object.frame()).unwrap();
        let event = decode_signed_audit_event(&plaintext).unwrap();
        assert_eq!(event.event().trace_id(), audited_access_trace(0x63));
        assert_eq!(event.event().timestamp_ms(), 431);
        assert_eq!(session.active.last_device_counter(), 3);
    }

    #[test]
    fn audited_login_edit_records_precondition_host_failure_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x71);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            rich_login_document(item_id, "Edit audit", "original-secret"),
            440,
            add_randomness,
            &local,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(441, audited_access_randomness(0x72), &local)
        .unwrap();

        let missing_id = ItemId::new([0x99; 16]);
        let missing = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .prepare_audited_login_edit(missing_id, 442, audited_access_randomness(0x73), &local)
        .unwrap();
        assert_eq!(
            missing.into_preparation().err(),
            Some(ApplicationError::NotFound)
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemUpdate,
                AuditOutcomeV1::Failed,
                Some(missing_id),
                None,
            )
        );
        let expected_revision = session.current_item_revision(item_id).unwrap().unwrap();
        session
            .prepare_audited_login_edit(item_id, 443, audited_access_randomness(0x74), &local)
            .unwrap()
            .into_preparation()
            .unwrap()
            .record_audited_host_failure(&local)
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemUpdate,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(expected_revision),
            )
        );
        let edited = session
            .prepare_audited_login_edit(item_id, 444, audited_access_randomness(0x75), &local)
            .unwrap()
            .into_preparation()
            .unwrap()
            .complete_audited(
                LoginEditInputV1::new(
                    Zeroizing::new("Edited".to_owned()),
                    Zeroizing::new("edited@example.test".to_owned()),
                    Zeroizing::new("replacement-secret".to_owned()),
                    vec![
                        Zeroizing::new("https://replacement.example.test".to_owned()),
                        Zeroizing::new("https://backup.example.test".to_owned()),
                    ],
                    Some(Zeroizing::new("replacement private notes".to_owned())),
                ),
                replace_item_randomness(0x76),
                &local,
            )
            .unwrap();
        assert!(edited.operation_succeeded());

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemUpdate,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(expected_revision),
            )
        );
        let current_revision = session.current_item_revision(item_id).unwrap().unwrap();
        let current = session.reveal_item_revision(current_revision).unwrap();
        let AnyRecord::Login(login) = current.payload() else {
            panic!("edited record must remain a login")
        };
        assert_eq!(
            login.urls,
            [
                "https://replacement.example.test",
                "https://backup.example.test"
            ]
        );
        assert_eq!(login.notes.as_deref(), Some("replacement private notes"));
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 6);
        assert_eq!(report.audit_event_count(), 4);
    }

    #[test]
    fn delete_item_publishes_one_parent_tombstone_and_rejects_repeat_delete() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x21);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Delete me", "secret"),
            450,
            add_randomness,
            &local,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let expected_revision = session.current_catalog.items[&item_id][0].revision_id();
        let prior_heads = session.local_pins().clone();
        let active = session
            .delete_item(
                expected_revision,
                451,
                452,
                delete_item_randomness(0x31),
                &local,
            )
            .unwrap();
        assert_eq!(active.last_device_counter(), 3);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 1);
        assert_eq!(reopened.search_item_count(), 0);
        assert_eq!(reopened.get_item(item_id).unwrap(), None);
        assert!(reopened.list_items().unwrap().is_empty());
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("deletion must become the sole current candidate")
        };
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([expected_revision])
        );
        let ItemState::Tombstone(tombstone) = candidate.state() else {
            panic!("deletion must materialize a tombstone")
        };
        assert_eq!(tombstone.item_id, item_id);
        assert_eq!(tombstone.deleted_at_ms, 451);
        let tombstone_revision = candidate.revision_id();
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 452);
        let exact_deleted = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            reopened.delete_item(
                tombstone_revision,
                453,
                454,
                delete_item_randomness(0x41),
                &local,
            ),
            Err(ApplicationError::ConflictRequired)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
        );
    }

    #[test]
    fn audited_current_delete_records_success_and_repeat_failure() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x51);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Audited delete", "secret"),
            455,
            add_randomness,
            &local,
        )
        .unwrap();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .activate_audit_epoch(456, audited_access_randomness(0x52), &local)
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let expected_revision = session.current_item_revision(item_id).unwrap().unwrap();
        let deleted = session
            .audited_delete_current_item(
                item_id,
                457,
                458,
                delete_item_randomness(0x53),
                audited_access_randomness(0x54),
                &local,
            )
            .unwrap();
        assert!(deleted.operation_succeeded());

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemDelete,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(expected_revision),
            )
        );
        let repeated = session
            .audited_delete_current_item(
                item_id,
                459,
                460,
                delete_item_randomness(0x55),
                audited_access_randomness(0x56),
                &local,
            )
            .unwrap();
        assert_eq!(repeated.into_operation(), Err(ApplicationError::NotFound));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemDelete,
                AuditOutcomeV1::Failed,
                Some(item_id),
                None,
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 5);
        assert_eq!(report.audit_event_count(), 3);
    }

    #[test]
    fn delete_item_rejects_missing_revision_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .delete_item(
                RevisionId::new([0x51; 32]),
                455,
                456,
                delete_item_randomness(0x51),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_active.as_slice())
        );
    }

    #[test]
    fn item_history_materializes_live_and_deleted_revisions_in_ancestry_order() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x61);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "History one", "first-secret"),
            500,
            add_randomness,
            &local,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let first_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                first_revision,
                new_login_document(item_id, "History two", "second-secret"),
                501,
                replace_item_randomness(0x71),
                &local,
            )
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let second_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .delete_item(
                second_revision,
                502,
                503,
                delete_item_randomness(0x81),
                &local,
            )
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let history = session
            .item_history(item_id, DEFAULT_ITEM_HISTORY_LIMIT)
            .unwrap();

        assert_eq!(history.len(), 3);
        assert_eq!(history[0].revision_id(), tombstone_revision);
        assert!(history[0].is_deleted());
        assert_eq!(history[0].causal_parent_count(), 1);
        assert_eq!(history[0].advisory_time_ms(), 502);
        assert_eq!(history[1].revision_id(), second_revision);
        assert_eq!(history[1].causal_parent_count(), 1);
        assert_eq!(history[2].revision_id(), first_revision);
        assert_eq!(history[2].causal_parent_count(), 0);
        let Some(RedactedItemView {
            record: RedactedRecordView::Login { title, .. },
            ..
        }) = history[1].redacted_item()
        else {
            panic!("historical live revision must retain safe login metadata")
        };
        assert_eq!(title, "History two");
        assert!(!format!("{:?}", history[1]).contains("History two"));

        let limited = session.item_history(item_id, 2).unwrap();
        assert_eq!(
            limited
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            vec![tombstone_revision, second_revision]
        );
        assert!(session
            .item_history(ItemId::new([0xff; 16]), 100)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn item_history_rejects_invalid_bounds_without_disclosing_identity() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let item_id = ItemId::new([0x91; 16]);

        assert_eq!(
            session.item_history(item_id, 0),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            session.item_history(item_id, MAX_ITEM_HISTORY_LIMIT + 1),
            Err(ApplicationError::BoundExceeded)
        );
    }

    #[test]
    fn reveal_item_revision_is_exact_reachable_live_and_zeroizing() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x92);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Reveal original", "original-secret"),
            500,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let original_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                original_revision,
                new_login_document(item_id, "Reveal current", "current-secret"),
                501,
                replace_item_randomness(0x93),
                &local,
            )
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let current_revision = session.current_catalog.items[&item_id][0].revision_id();
        let original = session.reveal_item_revision(original_revision).unwrap();
        let AnyRecord::Login(login) = original.payload() else {
            panic!("fixture must reveal a login")
        };
        assert_eq!(login.title, "Reveal original");
        assert_eq!(login.password, "original-secret");

        let revealed = session
            .reveal_item_revision_field(
                original_revision,
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::Clipboard,
            )
            .unwrap();
        assert_eq!(revealed.as_bytes(), b"original-secret");
        assert_eq!(revealed.encoding(), crate::RevealedSecretEncodingV1::Utf8);
        assert!(matches!(
            session.reveal_item_revision_field(
                original_revision,
                SecretFieldV1::CardCvv,
                SecretDisclosureIntentV1::Clipboard,
            ),
            Err(ApplicationError::InvalidInput)
        ));
        assert!(matches!(
            session.reveal_item_revision_field(
                RevisionId::new([0x94; 32]),
                SecretFieldV1::LoginPassword,
                SecretDisclosureIntentV1::InteractiveReveal { confirmed: false },
            ),
            Err(ApplicationError::InvalidInput)
        ));

        let mut current = session.reveal_item_revision(current_revision).unwrap();
        let AnyRecord::Login(login) = current.payload() else {
            panic!("fixture must reveal a login")
        };
        assert_eq!(login.password, "current-secret");
        current.zeroize();
        let AnyRecord::Login(login) = current.payload() else {
            panic!("zeroized fixture must retain its record variant")
        };
        assert!(login.password.is_empty());
        assert!(login.title.is_empty());
        assert!(matches!(
            session.reveal_item_revision(RevisionId::new([0x94; 32])),
            Err(ApplicationError::NotFound)
        ));

        session
            .delete_item(
                current_revision,
                502,
                503,
                delete_item_randomness(0x95),
                &local,
            )
            .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        assert!(matches!(
            session.reveal_item_revision(tombstone_revision),
            Err(ApplicationError::InvalidInput)
        ));
    }

    #[test]
    fn restore_item_copies_one_reachable_live_revision_without_rewinding_heads() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xa1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Restore me", "original-secret"),
            600,
            add_randomness,
            &local,
        )
        .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let original_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .replace_item(
                original_revision,
                new_login_document(item_id, "Changed", "changed-secret"),
                601,
                replace_item_randomness(0xb1),
                &local,
            )
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let changed_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .delete_item(
                changed_revision,
                602,
                603,
                delete_item_randomness(0xc1),
                &local,
            )
            .unwrap();

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let prior_heads = session.local_pins().clone();
        let active = session
            .restore_item(
                original_revision,
                604,
                restore_item_randomness(0xd1),
                &local,
            )
            .unwrap();
        assert_eq!(active.last_device_counter(), 5);

        let reopened = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let [candidate] = reopened.current_catalog.items[&item_id].as_slice() else {
            panic!("restoration must become the sole current candidate")
        };
        let restored_revision = candidate.revision_id();
        assert_ne!(restored_revision, original_revision);
        assert_eq!(
            candidate.causal_parents(),
            &BTreeSet::from([original_revision])
        );
        let ItemState::Live(document) = candidate.state() else {
            panic!("restoration must create a live revision")
        };
        let AnyRecord::Login(login) = document.payload() else {
            panic!("restoration must preserve the selected schema")
        };
        assert_eq!(login.title, "Restore me");
        assert_eq!(login.password, "original-secret");
        let history = reopened.item_history(item_id, 100).unwrap();
        assert_eq!(
            history
                .iter()
                .map(ItemHistoryViewV1::revision_id)
                .collect::<Vec<_>>(),
            vec![
                restored_revision,
                tombstone_revision,
                changed_revision,
                original_revision
            ]
        );
        let head = *reopened.open_report().heads().iter().next().unwrap();
        let commit = reopened._repository.read_commit(head).unwrap();
        assert_eq!(
            commit.parents(),
            prior_heads.iter().copied().collect::<Vec<_>>()
        );
        assert_eq!(commit.added_objects().len(), 2);
        assert_eq!(commit.wall_time_ms(), 604);
    }

    #[test]
    fn restore_item_rejects_missing_current_and_tombstone_selections_before_cas() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0xe1);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Restore guards", "secret"),
            610,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let live_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_live = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.restore_item(live_revision, 611, restore_item_randomness(0xf1), &local,),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_live.as_slice())
        );

        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .delete_item(
            live_revision,
            612,
            613,
            delete_item_randomness(0x01),
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        let exact_deleted = local.0.lock().unwrap().clone().unwrap();
        assert_eq!(
            session.restore_item(
                tombstone_revision,
                614,
                restore_item_randomness(0x11),
                &local,
            ),
            Err(ApplicationError::InvalidInput)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
        );

        assert_eq!(
            open_active_vault(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap()
            .restore_item(
                RevisionId::new([0x7f; 32]),
                615,
                restore_item_randomness(0x21),
                &local,
            ),
            Err(ApplicationError::NotFound)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_deleted.as_slice())
        );
    }

    #[test]
    fn audited_item_bound_restore_records_selection_failures_and_success() {
        let (locator, local, bootstrap, factory) = initialized();
        let add_randomness = add_item_randomness(0x31);
        let item_id = add_randomness.item_id();
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap()
        .add_item(
            new_login_document(item_id, "Restore audit", "original-secret"),
            620,
            add_randomness,
            &local,
        )
        .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let original_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .delete_item(
                original_revision,
                621,
                622,
                delete_item_randomness(0x32),
                &local,
            )
            .unwrap();
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let tombstone_revision = session.current_catalog.items[&item_id][0].revision_id();
        session
            .activate_audit_epoch(623, audited_access_randomness(0x33), &local)
            .unwrap();

        let wrong_item_id = ItemId::new([0x91; 16]);
        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        let wrong_item = session
            .audited_restore_item_for_item(
                wrong_item_id,
                original_revision,
                100,
                624,
                restore_item_randomness(0x34),
                audited_access_randomness(0x35),
                &local,
            )
            .unwrap();
        assert_eq!(wrong_item.into_operation(), Err(ApplicationError::NotFound));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRestore,
                AuditOutcomeV1::Failed,
                Some(wrong_item_id),
                None,
            )
        );
        let tombstone = session
            .audited_restore_item_for_item(
                item_id,
                tombstone_revision,
                100,
                625,
                restore_item_randomness(0x36),
                audited_access_randomness(0x37),
                &local,
            )
            .unwrap();
        assert_eq!(
            tombstone.into_operation(),
            Err(ApplicationError::InvalidInput)
        );

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRestore,
                AuditOutcomeV1::Failed,
                Some(item_id),
                Some(tombstone_revision),
            )
        );
        let restored = session
            .audited_restore_item_for_item(
                item_id,
                original_revision,
                100,
                626,
                restore_item_randomness(0x38),
                audited_access_randomness(0x39),
                &local,
            )
            .unwrap();
        assert!(restored.operation_succeeded());

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::ItemRestore,
                AuditOutcomeV1::Succeeded,
                Some(item_id),
                Some(original_revision),
            )
        );
        let report = session.audit_verify().unwrap();
        assert_eq!(report.commit_count(), 7);
        assert_eq!(report.audit_event_count(), 4);
    }

    #[test]
    fn pending_publication_replays_exactly_and_advances_active_state() {
        let (locator, local, bootstrap, factory) = initialized();
        let publication = install_pending(&local);

        let active = recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert_eq!(active.last_device_counter(), 2);
        assert_eq!(active.catalog_root(), publication.catalog_root());
        assert!(matches!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(_)
        ));

        let session = open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(session.open_report().commit_count(), 2);
        assert_eq!(session.open_report().heads(), publication.expected_heads());
    }

    #[test]
    fn pending_recovery_retains_exact_journal_across_ambiguous_provider_failure() {
        let passphrase = b"active passphrase";
        let prepared = prepare_generation_zero(
            Zeroizing::new(passphrase.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            randomness(),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(FaultInjectingObjectStore::new(InMemoryObjectStore::new()));
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();
        let publication = install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        backend
            .enqueue(FaultAction {
                operation: StoreOperation::PutImmutable,
                effect: FaultEffect::CommitPutThenNetwork,
            })
            .unwrap();

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(passphrase.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );

        let active = recover_pending_publication(
            Zeroizing::new(passphrase.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert_eq!(backend.pending_faults().unwrap(), 0);
    }

    #[test]
    fn pending_recovery_authenticates_before_any_repository_effect() {
        let (locator, local, bootstrap, factory) = initialized();
        install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(b"wrong".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );
    }

    #[test]
    fn pending_recovery_accepts_only_an_identical_concurrent_active_winner() {
        let (locator, local, bootstrap, factory) = initialized();
        let publication = install_pending(&local);
        local.concurrent_winner_on_next_compare();

        let active = recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(active.pinned_heads(), publication.expected_heads());
        assert!(matches!(
            LocalVaultStateV1::decode(&local.0.lock().unwrap().clone().unwrap()).unwrap(),
            LocalVaultStateV1::Active(_)
        ));
    }

    #[test]
    fn pending_recovery_retains_journal_when_final_local_commit_fails() {
        let (locator, local, bootstrap, factory) = initialized();
        install_pending(&local);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();
        local.fail_next_compare(LocalStateStoreError::Unavailable);

        assert_eq!(
            recover_pending_publication(
                Zeroizing::new(b"active passphrase".to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(
            local.0.lock().unwrap().as_deref(),
            Some(exact_pending.as_slice())
        );

        recover_pending_publication(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
    }

    // ─────────────────────────────────────────────────────────────────
    // Oversized records fail closed on every mutating path
    //
    // These pin the *user-facing* half of the fix. The codec-level
    // boundary lives in `codec.rs`; what matters here is that a record
    // the encoder will not emit produces a closed `ApplicationError` out
    // of the real ceremonies rather than aborting the process.
    //
    // A password of 2 MiB is the shape used throughout: comfortably
    // inside this crate's 16 MiB plaintext gate, comfortably outside
    // canonical-CBOR's 1 MiB encoded ceiling.
    // ─────────────────────────────────────────────────────────────────

    const OVERSIZED_PASSWORD_BYTES: usize = 2 * 1024 * 1024;

    fn oversized_password() -> String {
        "p".repeat(OVERSIZED_PASSWORD_BYTES)
    }

    fn active_session(
        locator: BootstrapLocator,
        local: &MemoryLocalStateStore,
        bootstrap: &MemoryBootstrapStore,
        factory: &V1ApplicationRepositoryFactory<InMemoryObjectStore>,
    ) -> UnlockedVaultV1 {
        open_active_vault(
            Zeroizing::new(b"active passphrase".to_vec()),
            locator,
            local,
            bootstrap,
            factory,
        )
        .unwrap()
    }

    #[test]
    fn add_item_reports_an_oversized_record_instead_of_aborting() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = active_session(locator, &local, &bootstrap, &factory);
        let randomness = add_item_randomness(0x41);
        let item_id = randomness.item_id();

        // The document itself is constructible -- the domain layer has no
        // field-length bound -- so the refusal has to come from the
        // publish path, and it has to be a value, not a crash.
        let document = new_login_document(item_id, "Oversized", &oversized_password());
        assert_eq!(
            session
                .add_item(document, 301, randomness, &local)
                .map(|_| ()),
            Err(ApplicationError::BoundExceeded)
        );

        // Nothing was written: the vault is still openable and still empty.
        let session = active_session(locator, &local, &bootstrap, &factory);
        assert!(session.current_catalog.items.is_empty());
    }

    #[test]
    fn login_edit_reports_an_oversized_replacement_instead_of_aborting() {
        let (locator, local, bootstrap, factory) = initialized();
        let session = active_session(locator, &local, &bootstrap, &factory);
        let randomness = add_item_randomness(0x42);
        let item_id = randomness.item_id();
        session
            .add_item(
                rich_login_document(item_id, "Editable", "short-password"),
                301,
                randomness,
                &local,
            )
            .unwrap();

        let session = active_session(locator, &local, &bootstrap, &factory);
        let preparation = session.prepare_login_edit(item_id).unwrap();
        assert_eq!(
            preparation
                .complete(
                    LoginEditInputV1::new(
                        Zeroizing::new("Editable".to_owned()),
                        Zeroizing::new("user@example.test".to_owned()),
                        Zeroizing::new(oversized_password()),
                        vec![Zeroizing::new("https://one.example.test".to_owned())],
                        None,
                    ),
                    302,
                    replace_item_randomness(0x43),
                    &local,
                )
                .map(|_| ()),
            Err(ApplicationError::BoundExceeded)
        );

        // The original revision survives the refused edit untouched.
        let session = active_session(locator, &local, &bootstrap, &factory);
        assert_eq!(session.current_catalog.items[&item_id].len(), 1);
    }

    #[test]
    fn portable_export_reports_an_oversized_artifact_instead_of_aborting() {
        // Each record here is under the codec's 1 MiB ceiling on its own,
        // so every item publishes cleanly. The export snapshot
        // concatenates them all, which is what crosses the ceiling -- and
        // that needs no hostile peer, just an ordinary vault with a
        // couple of large entries in it.
        let (locator, local, bootstrap, factory) = initialized();
        let large = "p".repeat(600 * 1024);
        for seed in [0x44_u8, 0x45] {
            let session = active_session(locator, &local, &bootstrap, &factory);
            let randomness = add_item_randomness(seed);
            let item_id = randomness.item_id();
            session
                .add_item(
                    new_login_document(item_id, "Large", &large),
                    301,
                    randomness,
                    &local,
                )
                .unwrap();
        }

        let session = active_session(locator, &local, &bootstrap, &factory);
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        assert_eq!(
            session
                .export_portable_with_passphrase(
                    &exact_bootstrap,
                    Zeroizing::new(b"separate export passphrase".to_vec()),
                    crate::PortableExportPolicyV1::new(8 * 1024, 1, 1).unwrap(),
                    crate::PortableExportRandomnessV1::new(
                        [0x5b; crate::PORTABLE_EXPORT_RANDOM_BYTES]
                    ),
                )
                .map(|_| ()),
            Err(ApplicationError::BoundExceeded)
        );

        // The vault is untouched and still openable after the refusal.
        let session = active_session(locator, &local, &bootstrap, &factory);
        assert_eq!(session.current_catalog.items.len(), 2);
    }

    // ─────────────────────────────────────────────────────────────────
    // A synced peer's oversized opaque record must not deny vault open
    //
    // These pin the invariant that vault open never fails because of one
    // item. The record below is authored the way a peer with a larger
    // framing budget authors it — directly as wire bytes, because this
    // product's own encoder refuses to emit it — and it arrives through
    // the shared object store the way a sync delivers it.
    //
    // 1.5 MiB is the size that matters: comfortably above canonical-
    // CBOR's 1 MiB encoded ceiling, comfortably below this crate's 16 MiB
    // plaintext gate. Records in that band decode and must stay decodable.
    // ─────────────────────────────────────────────────────────────────

    const PEER_OPAQUE_PAYLOAD_BYTES: usize = 1_536 * 1024;

    /// Emit a CBOR header for `major` carrying `len`, in the smallest form
    /// — the same choice [`coding_adventures_canonical_cbor::encode`] makes,
    /// written out here because the peer's encoder is not ours.
    fn cbor_header(major: u8, len: usize) -> Vec<u8> {
        let tag = major << 5;
        let len = len as u64;
        match len {
            0..=23 => vec![tag | len as u8],
            24..=0xFF => vec![tag | 24, len as u8],
            0x100..=0xFFFF => {
                let mut out = vec![tag | 25];
                out.extend_from_slice(&(len as u16).to_be_bytes());
                out
            }
            _ => {
                let mut out = vec![tag | 26];
                out.extend_from_slice(&(len as u32).to_be_bytes());
                out
            }
        }
    }

    /// One opaque payload: canonical CBOR for a byte string of `len` bytes.
    ///
    /// This is the shape `AnyRecord::Opaque::payload_bytes` carries — the
    /// encoded `d` value, not the value's contents.
    fn peer_opaque_payload(len: usize) -> Vec<u8> {
        let mut payload = cbor_header(2, len);
        payload.extend(std::iter::repeat_n(0x5a_u8, len));
        payload
    }

    /// The record envelope a peer writes: a two-entry map `{d: payload,
    /// t: content_type}`. Canonical map order is length-first then
    /// bytewise, and the two keys are both one character, so `"d"`
    /// (`0x61 0x64`) precedes `"t"` (`0x61 0x74`).
    fn peer_opaque_record(content_type: &str, payload: &[u8]) -> Vec<u8> {
        let mut wire = vec![0xa2, 0x61, b'd'];
        wire.extend_from_slice(payload);
        wire.extend_from_slice(&[0x61, b't']);
        wire.extend_from_slice(&cbor_header(3, content_type.len()));
        wire.extend_from_slice(content_type.as_bytes());
        wire
    }

    /// Replace the one occurrence of `needle` in `haystack`.
    ///
    /// Splicing works here without repairing any enclosing length because
    /// CBOR maps and arrays are prefixed with an element *count*, not a
    /// byte length: growing a value nested inside them leaves every
    /// enclosing header correct.
    fn splice_only_occurrence(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
        let at = haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("the placeholder record must appear in the revision plaintext");
        assert!(
            haystack[at + 1..]
                .windows(needle.len())
                .all(|window| window != needle),
            "the placeholder record must appear exactly once",
        );
        let mut out = Vec::with_capacity(haystack.len() - needle.len() + replacement.len());
        out.extend_from_slice(&haystack[..at]);
        out.extend_from_slice(replacement);
        out.extend_from_slice(&haystack[at + needle.len()..]);
        out
    }

    /// The revision plaintext a peer seals for one live opaque item whose
    /// payload is `payload_bytes` long.
    ///
    /// Built by encoding the revision with a small placeholder record
    /// through the product's own encoder, then swapping the placeholder's
    /// byte-string field for the oversized one. Everything except the one
    /// record therefore has exactly the bytes this product would write.
    fn peer_opaque_revision_plaintext(item_id: ItemId, payload_bytes: usize) -> Vec<u8> {
        let placeholder_payload = peer_opaque_payload(8);
        let placeholder_record =
            encode_opaque(FIXTURE_OPAQUE_CONTENT_TYPE, &placeholder_payload).unwrap();
        let document = ItemDocument::new(
            item_id,
            ContentType::new(FIXTURE_OPAQUE_CONTENT_TYPE).unwrap(),
            500,
            500,
            LwwRegister::new(true, 500, OperationId::new([0xf0; 32])),
            ObservedSet::new(),
            ObservedSet::new(),
            AnyRecord::Opaque {
                content_type: FIXTURE_OPAQUE_CONTENT_TYPE.to_owned(),
                payload_bytes: placeholder_payload,
            },
            ObservedSet::new(),
        )
        .unwrap();
        let candidate = ItemCandidate::new(
            RevisionId::new([0; 32]),
            [],
            ItemState::Live(Box::new(document)),
        )
        .unwrap();
        let plaintext =
            encode_item_revision(candidate.causal_parents(), candidate.state()).unwrap();

        let oversized_record = peer_opaque_record(
            FIXTURE_OPAQUE_CONTENT_TYPE,
            &peer_opaque_payload(payload_bytes),
        );
        let mut placeholder_field = cbor_header(2, placeholder_record.len());
        placeholder_field.extend_from_slice(&placeholder_record);
        let mut oversized_field = cbor_header(2, oversized_record.len());
        oversized_field.extend_from_slice(&oversized_record);
        splice_only_occurrence(&plaintext, &placeholder_field, &oversized_field)
    }

    /// Publish one peer-authored commit straight into the shared object
    /// store, then adopt its head locally.
    ///
    /// The ordinary local mutation path cannot deliver this record: it
    /// stages the whole publication journal -- sealed frames included --
    /// in local state, and a 1.5 MiB frame is past what the local-state
    /// encode will write. That is the point. A record in this size band
    /// can only have been authored somewhere with a larger framing
    /// budget, so the fixture writes it the way that device would: the
    /// objects land in the shared store and the head moves, with local
    /// state carrying only the pin.
    fn peer_publishes(
        locator: BootstrapLocator,
        local: &MemoryLocalStateStore,
        bootstrap: &MemoryBootstrapStore,
        factory: &V1ApplicationRepositoryFactory<InMemoryObjectStore>,
        publication: PublicationJournalV1,
    ) {
        let exact_state = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_state).unwrap()
        else {
            panic!("fixture must be active")
        };
        assert_eq!(active.bootstrap_locator(), locator);
        let exact_bootstrap = bootstrap.latest_bytes().unwrap();
        let material = unlock_active_material(
            Zeroizing::new(b"active passphrase".to_vec()),
            &active,
            &exact_bootstrap,
        )
        .unwrap();
        let repository = factory
            .connect(material.repository_address, Box::new(material.verifier))
            .unwrap();
        repository.initialize().unwrap();
        let adopted = active.after_publication(&publication).unwrap();
        let receipt = repository
            .publish(publication.publication(), publication.base_heads())
            .unwrap();
        assert_eq!(receipt.heads(), publication.expected_heads());
        // Adopting the head is what any sync ceremony leaves behind, and
        // the mutation paths require the pin to match the observed head
        // before they will write. Writing the adopted state directly keeps
        // this fixture about the poisoned record rather than about a
        // ceremony this slice does not yet have.
        *local.0.lock().unwrap() = Some(LocalVaultStateV1::Active(adopted).encode().unwrap());
    }

    /// A peer commit carrying one live opaque item whose payload is
    /// `payload_bytes` long.
    fn peer_opaque_publication(
        active: &ActiveStateV1,
        item_id: ItemId,
        payload_bytes: usize,
    ) -> (PublicationJournalV1, RevisionId) {
        let fixture = generation_zero_bytes();
        let root_key: [u8; 32] = fixture[48..80].try_into().unwrap();
        let keys = V1Keys::derive(active.vault_id(), &root_key).unwrap();
        let frame = seal_object(
            &keys,
            ObjectKind::ItemRevision,
            &peer_opaque_revision_plaintext(item_id, payload_bytes),
            &ObjectRandomness::new([0xc1; 32], [0xc2; 24], [0xc3; 24]),
        )
        .unwrap();
        let revision_id = RevisionId::new(*frame.id().unwrap().as_bytes());
        let catalog = CatalogV1::new(BTreeMap::from([(item_id, vec![revision_id])])).unwrap();
        let catalog_frame = seal_object(
            &keys,
            ObjectKind::Catalog,
            &catalog.encode().unwrap(),
            &ObjectRandomness::new([0xc4; 32], [0xc5; 24], [0xc6; 24]),
        )
        .unwrap();
        (
            publication_for_catalog(active, vec![frame], catalog_frame),
            revision_id,
        )
    }

    #[test]
    fn the_peer_record_encoder_agrees_with_this_products_own() {
        // The fixture above is only a faithful model of a larger-budget peer
        // if it writes the same bytes this product writes wherever this
        // product is willing to write at all.
        for len in [0, 8, 23, 24, 255, 256, 65_535, 65_536] {
            let payload = peer_opaque_payload(len);
            assert_eq!(
                peer_opaque_record(FIXTURE_OPAQUE_CONTENT_TYPE, &payload),
                encode_opaque(FIXTURE_OPAQUE_CONTENT_TYPE, &payload).unwrap(),
                "peer and product must agree at payload length {len}",
            );
        }
    }

    /// Sync one peer-authored opaque item of `payload_bytes` into a fresh
    /// vault and return everything needed to reopen it.
    #[allow(clippy::type_complexity)]
    fn vault_with_synced_opaque_item(
        payload_bytes: usize,
    ) -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
        ItemId,
        RevisionId,
    ) {
        let (locator, local, bootstrap, factory) = initialized();
        let exact_active = local.0.lock().unwrap().clone().unwrap();
        let LocalVaultStateV1::Active(active) = LocalVaultStateV1::decode(&exact_active).unwrap()
        else {
            panic!("fixture must be active")
        };
        let item_id = ItemId::new([0x5c; 16]);
        let (publication, revision_id) = peer_opaque_publication(&active, item_id, payload_bytes);
        peer_publishes(locator, &local, &bootstrap, &factory, publication);
        (locator, local, bootstrap, factory, item_id, revision_id)
    }

    #[test]
    fn a_synced_opaque_record_under_the_encode_ceiling_opens() {
        // The control for the test below. Same fixture, same delivery, only
        // the payload size differs -- so a failure there is the size band
        // and not the way the record was authored or synced.
        let (locator, local, bootstrap, factory, item_id, revision_id) =
            vault_with_synced_opaque_item(512 * 1024);
        let session = active_session(locator, &local, &bootstrap, &factory);
        let candidates = &session.current_catalog.items[&item_id];
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].revision_id(), revision_id);
    }

    #[test]
    fn a_synced_oversized_opaque_record_leaves_the_vault_openable() {
        // Before the fix this was the worst failure in the codec: the opaque
        // arm of `decode_record` re-encoded the payload it had just decoded,
        // the re-encode hit the 1 MiB ceiling, and the resulting error rose
        // through `decode_item_revision`, `read_candidate`, and
        // `materialize_current_catalog` to deny `open_active_vault`. One
        // synced record locked the whole vault, permanently, with no session
        // to delete it from.
        let (locator, local, bootstrap, factory, item_id, revision_id) =
            vault_with_synced_opaque_item(PEER_OPAQUE_PAYLOAD_BYTES);

        let session = active_session(locator, &local, &bootstrap, &factory);
        let candidates = &session.current_catalog.items[&item_id];
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].revision_id(), revision_id);

        // The payload survives the open intact: the decoder hands back the
        // record's own bytes, so what the peer sealed is what is read.
        let ItemState::Live(document) = candidates[0].state() else {
            panic!("the synced candidate must be live")
        };
        let AnyRecord::Opaque {
            content_type,
            payload_bytes,
        } = document.payload()
        else {
            panic!("an unknown content type must decode as opaque")
        };
        assert_eq!(content_type, FIXTURE_OPAQUE_CONTENT_TYPE);
        assert_eq!(
            payload_bytes.as_slice(),
            peer_opaque_payload(PEER_OPAQUE_PAYLOAD_BYTES).as_slice(),
        );
    }

    #[test]
    fn a_synced_oversized_opaque_item_can_be_deleted() {
        // The escape hatch. Opening is only half of it: the operator has to
        // be able to get rid of the record, and deletion writes a tombstone,
        // which never re-encodes the poisoned payload.
        let (locator, local, bootstrap, factory, item_id, _) =
            vault_with_synced_opaque_item(PEER_OPAQUE_PAYLOAD_BYTES);

        let session = active_session(locator, &local, &bootstrap, &factory);
        session
            .delete_current_item(item_id, 600, 601, delete_item_randomness(0x5d), &local)
            .unwrap();

        let session = active_session(locator, &local, &bootstrap, &factory);
        let candidates = &session.current_catalog.items[&item_id];
        assert_eq!(candidates.len(), 1);
        assert!(matches!(candidates[0].state(), ItemState::Tombstone(_)));
    }

    // ---------------------------------------------------------------------
    // VLT-PM43 — passphrase rotation
    // ---------------------------------------------------------------------

    const OLD_PASSPHRASE: &[u8] = b"active passphrase";
    const NEW_PASSPHRASE: &[u8] = b"rotated passphrase, deliberately different";

    fn rotation_policy() -> PassphraseRotationPolicyV1 {
        PassphraseRotationPolicyV1::new(8 * 1024, 1, 1).unwrap()
    }

    fn rotation_randomness(seed: u8) -> PassphraseRotationRandomnessV1 {
        let mut bytes = [0; PASSPHRASE_ROTATION_RANDOM_BYTES];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(53).wrapping_add(seed);
        }
        PassphraseRotationRandomnessV1::new(bytes)
    }

    /// The vault root key the generation-zero fixture drew.
    ///
    /// Reconstructed from the same deterministic block `initialized_with`
    /// feeds the preparation, at the offset `prepare_generation_zero` reads it
    /// from: 32 locator bytes, then 16 vault-ID bytes, then the root key.
    fn fixture_root_key() -> [u8; 32] {
        generation_zero_bytes()[48..80].try_into().unwrap()
    }

    fn fixture_local_secret(active: &ActiveStateV1) -> LocalSecretV1 {
        let keys = V1Keys::derive(active.vault_id(), &fixture_root_key()).unwrap();
        crate::open_local_secret(&keys, active.local_secret()).unwrap()
    }

    /// One initialized vault holding one login, with the backend shared so a
    /// test can watch the repository's complete change feed.
    fn vault_with_one_item() -> (
        BootstrapLocator,
        MemoryLocalStateStore,
        MemoryBootstrapStore,
        V1ApplicationRepositoryFactory<InMemoryObjectStore>,
        Arc<InMemoryObjectStore>,
        ItemId,
    ) {
        let prepared = prepare_generation_zero(
            Zeroizing::new(OLD_PASSPHRASE.to_vec()),
            GenerationZeroPolicyV1::new(8 * 1024, 1, 1, 10).unwrap(),
            GenerationZeroRandomness::new(generation_zero_bytes()),
        )
        .unwrap();
        let locator = prepared.bootstrap_locator();
        let local = MemoryLocalStateStore::default();
        let bootstrap = MemoryBootstrapStore::default();
        let backend = Arc::new(InMemoryObjectStore::new());
        let factory = V1ApplicationRepositoryFactory::from_shared(Arc::clone(&backend));
        complete_generation_zero(prepared, &local, &bootstrap, &factory).unwrap();

        let randomness = add_item_randomness(0x21);
        let item_id = randomness.item_id();
        active_session(locator, &local, &bootstrap, &factory)
            .add_item(
                new_login_document(item_id, "rotation fixture", "fixture password"),
                400,
                randomness,
                &local,
            )
            .unwrap();
        (locator, local, bootstrap, factory, backend, item_id)
    }

    /// Every object this repository has ever been asked to write, in order.
    ///
    /// This is the measurement behind §14.8's "without re-encrypting every item
    /// body". Comparing object *counts* would only show that the number of
    /// objects stayed the same; comparing the whole change feed shows that no
    /// write happened at all, which is the actual claim.
    fn repository_change_feed(backend: &InMemoryObjectStore) -> Vec<(u64, String)> {
        backend
            .changes(None)
            .unwrap()
            .map(|page| {
                page.events
                    .into_iter()
                    .map(|event| (event.sequence, format!("{:?}", event.object)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn exact_bootstrap(bootstrap: &MemoryBootstrapStore) -> Vec<u8> {
        bootstrap.latest_bytes().unwrap()
    }

    fn durable_active(local: &MemoryLocalStateStore) -> ActiveStateV1 {
        let encoded = local.0.lock().unwrap().clone().unwrap();
        match LocalVaultStateV1::decode(&encoded).unwrap() {
            LocalVaultStateV1::Active(active) => active,
            _ => panic!("fixture must be active"),
        }
    }

    fn prepared_rotation(
        local: &MemoryLocalStateStore,
        bootstrap: &MemoryBootstrapStore,
        seed: u8,
    ) -> crate::PreparedPassphraseRotationV1 {
        let active = durable_active(local);
        let local_secret = fixture_local_secret(&active);
        crate::prepare_passphrase_rotation(
            &active,
            &exact_bootstrap(bootstrap),
            &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
            &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
            rotation_policy(),
            &rotation_randomness(seed),
            &local_secret,
        )
        .unwrap()
    }

    fn install_pending_rotation(
        local: &MemoryLocalStateStore,
        bootstrap: &MemoryBootstrapStore,
        seed: u8,
    ) {
        let prepared = prepared_rotation(local, bootstrap, seed);
        let journal =
            PendingRotationV1::new(durable_active(local), prepared.bootstrap().to_vec()).unwrap();
        *local.0.lock().unwrap() = Some(
            LocalVaultStateV1::PendingRotation(journal)
                .encode()
                .unwrap(),
        );
    }

    #[test]
    fn rotation_rewraps_the_root_key_and_writes_no_repository_object() {
        let (locator, local, bootstrap, factory, backend, item_id) = vault_with_one_item();
        let before_feed = repository_change_feed(&backend);
        assert!(
            !before_feed.is_empty(),
            "the fixture must have written objects"
        );
        let before_bootstrap = exact_bootstrap(&bootstrap);

        active_session(locator, &local, &bootstrap, &factory)
            .rotate_passphrase(
                &before_bootstrap,
                &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                rotation_policy(),
                rotation_randomness(0x31),
                &local,
                &bootstrap,
            )
            .unwrap();

        // The whole of the O(1) claim: not one repository object was written,
        // so no item body, catalog, commit, or certificate was re-encrypted.
        assert_eq!(repository_change_feed(&backend), before_feed);

        // The new bootstrap is a successor of the old one, under the same
        // authority and vault, with a different salt and a different wrap.
        let after_bootstrap = exact_bootstrap(&bootstrap);
        let before = BootstrapV1::decode(&before_bootstrap).unwrap();
        let after = BootstrapV1::decode(&after_bootstrap).unwrap();
        assert_eq!(after.generation, before.generation + 1);
        assert_eq!(after.previous_bootstrap, Some(before.id().unwrap()));
        assert_eq!(after.vault_id, before.vault_id);
        assert_eq!(after.authority_public_key, before.authority_public_key);
        assert_ne!(after.kdf.salt, before.kdf.salt);
        assert_ne!(
            after.passphrase_root_wrap.ciphertext,
            before.passphrase_root_wrap.ciphertext
        );
        assert_ne!(
            after.passphrase_root_wrap.nonce,
            before.passphrase_root_wrap.nonce
        );

        // The old passphrase is gone; the new one opens the same vault, with
        // the same item, decrypted by the same unchanged root key.
        assert_eq!(
            open_active_vault(
                Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        let reopened = open_active_vault(
            Zeroizing::new(NEW_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(reopened.item_count(), 1);
        assert!(reopened.get_item(item_id).unwrap().is_some());
    }

    #[test]
    fn rotation_durably_removes_the_retired_wrap() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        let before_bootstrap = exact_bootstrap(&bootstrap);

        active_session(locator, &local, &bootstrap, &factory)
            .rotate_passphrase(
                &before_bootstrap,
                &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                rotation_policy(),
                rotation_randomness(0x37),
                &local,
                &bootstrap,
            )
            .unwrap();

        // Advancing the pointer is not enough: the retired record still wraps
        // the same unchanged root key under the old passphrase, so leaving it
        // behind would make the rotation ceremonial.
        assert!(bootstrap.retired_bytes().is_empty());
    }

    #[test]
    fn a_wrong_current_passphrase_rotates_nothing() {
        let (locator, local, bootstrap, factory, backend, _) = vault_with_one_item();
        let before_feed = repository_change_feed(&backend);
        let before_bootstrap = exact_bootstrap(&bootstrap);
        let before_state = local.0.lock().unwrap().clone();
        let active = durable_active(&local);
        let local_secret = fixture_local_secret(&active);

        assert_eq!(
            crate::prepare_passphrase_rotation(
                &active,
                &before_bootstrap,
                &Zeroizing::new(b"not the current passphrase".to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                rotation_policy(),
                &rotation_randomness(0x41),
                &local_secret,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );

        assert_eq!(*local.0.lock().unwrap(), before_state);
        assert_eq!(exact_bootstrap(&bootstrap), before_bootstrap);
        assert_eq!(repository_change_feed(&backend), before_feed);
        assert!(open_active_vault(
            Zeroizing::new(OLD_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .is_ok());
    }

    #[test]
    fn a_rotation_bound_to_a_different_root_key_fails_closed() {
        let (_locator, local, bootstrap, _factory, _backend, _) = vault_with_one_item();
        let active = durable_active(&local);
        let mut foreign = *fixture_local_secret(&active).authority_seed();
        foreign[0] ^= 1;
        let foreign_secret = LocalSecretV1::new(
            active.vault_id(),
            active.device_id(),
            foreign,
            [0x11; 32],
            [0x22; 32],
        );

        assert_eq!(
            crate::prepare_passphrase_rotation(
                &active,
                &exact_bootstrap(&bootstrap),
                &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                rotation_policy(),
                &rotation_randomness(0x43),
                &foreign_secret,
            )
            .err(),
            Some(ApplicationError::IntegrityFailure)
        );
    }

    #[test]
    fn a_rotation_that_reaches_the_journal_rolls_forward_without_a_passphrase() {
        let (locator, local, bootstrap, factory, _backend, item_id) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0x47);

        // A locked reader sees a state the next command finishes, and says so
        // without asking for anything.
        assert_eq!(
            crate::VaultAccessV1::locked(locator)
                .status(&local)
                .unwrap()
                .state(),
            crate::VaultStatusStateV1::RecoveryRequired
        );
        assert_eq!(
            crate::VaultAccessV1::locked(locator)
                .doctor(&local, &bootstrap)
                .state(),
            crate::VaultDoctorStateV1::RecoveryRequired
        );

        // No passphrase reaches the roll-forward.
        crate::recover_pending_rotation(locator, &local, &bootstrap).unwrap();

        assert_eq!(
            open_active_vault(
                Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        let reopened = open_active_vault(
            Zeroizing::new(NEW_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert!(reopened.get_item(item_id).unwrap().is_some());
        assert!(bootstrap.retired_bytes().is_empty());
    }

    #[test]
    fn a_replayed_roll_forward_reaches_the_same_state_it_already_installed() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0x4b);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();

        let first = crate::recover_pending_rotation(locator, &local, &bootstrap).unwrap();
        let settled = local.0.lock().unwrap().clone().unwrap();

        // Replaying an already-finished rotation is not an error path a person
        // can reach — the state is `Active` again — so the honest assertion is
        // that the journal is gone and the finished state is the one the first
        // call returned.
        assert_eq!(
            crate::recover_pending_rotation(locator, &local, &bootstrap).err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_ne!(settled, exact_pending);
        assert_eq!(settled, LocalVaultStateV1::Active(first).encode().unwrap());

        // And re-running the *interrupted* half twice is sound: reinstall the
        // journal and finish it again against the already-advanced store.
        *local.0.lock().unwrap() = Some(exact_pending);
        crate::recover_pending_rotation(locator, &local, &bootstrap).unwrap();
        assert_eq!(local.0.lock().unwrap().clone().unwrap(), settled);
        assert!(open_active_vault(
            Zeroizing::new(NEW_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .is_ok());
    }

    #[test]
    fn an_interrupted_rotation_is_finished_by_an_ordinary_recovering_unlock() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0x53);

        let mut access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .unlock_recovering_pending_publication(
                    Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                    &local,
                    &bootstrap,
                    &factory,
                )
                .unwrap(),
            crate::UnlockRecoveryV1::RecoveredPendingRotation
        );
        assert!(access.is_unlocked());
    }

    #[test]
    fn the_old_passphrase_still_finishes_an_interrupted_rotation_then_is_refused() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0x59);

        // The roll-forward consumes no passphrase, so a person who types the
        // one they had before the crash still gets their vault repaired — and
        // then an honest refusal from the open that follows, naming the state
        // the vault is now actually in.
        let mut access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .unlock_recovering_pending_publication(
                    Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                    &local,
                    &bootstrap,
                    &factory,
                )
                .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        assert!(access.is_locked());
        assert_eq!(
            crate::VaultAccessV1::locked(locator)
                .status(&local)
                .unwrap()
                .state(),
            crate::VaultStatusStateV1::Locked
        );

        // Nothing is left to recover, and the new passphrase simply works.
        let mut access = crate::VaultAccessV1::locked(locator);
        assert_eq!(
            access
                .unlock_recovering_pending_publication(
                    Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                    &local,
                    &bootstrap,
                    &factory,
                )
                .unwrap(),
            crate::UnlockRecoveryV1::AlreadyActive
        );
        assert!(access.is_unlocked());
    }

    #[test]
    fn an_audited_rotation_publishes_its_event_before_the_effect() {
        let (locator, local, bootstrap, factory, backend, _) = vault_with_one_item();
        active_session(locator, &local, &bootstrap, &factory)
            .activate_audit_epoch(500, audited_access_randomness(0x61), &local)
            .unwrap();
        let before_feed = repository_change_feed(&backend);
        let before_bootstrap = exact_bootstrap(&bootstrap);

        active_session(locator, &local, &bootstrap, &factory)
            .audited_rotate_passphrase(
                &before_bootstrap,
                &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                rotation_policy(),
                rotation_randomness(0x67),
                600,
                audited_access_randomness(0x6b),
                &local,
                &bootstrap,
            )
            .unwrap();

        // An audited rotation does write to the repository — one audit-only
        // commit — and that is the only thing it adds. Every object written
        // before it is still there, unchanged and in the same order.
        let after_feed = repository_change_feed(&backend);
        assert_eq!(after_feed[..before_feed.len()], before_feed[..]);
        assert!(after_feed.len() > before_feed.len());

        let reopened = open_active_vault(
            Zeroizing::new(NEW_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert_eq!(
            latest_audit_facts(&reopened),
            (
                AuditActionV1::PassphraseRotate,
                AuditOutcomeV1::Succeeded,
                None,
                None
            )
        );
        assert!(reopened.audit_verify().unwrap().integrity_verified());
    }

    #[test]
    fn a_failed_audited_rotation_is_recorded_and_changes_no_wrap() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        active_session(locator, &local, &bootstrap, &factory)
            .activate_audit_epoch(500, audited_access_randomness(0x71), &local)
            .unwrap();
        let before_bootstrap = exact_bootstrap(&bootstrap);

        assert_eq!(
            active_session(locator, &local, &bootstrap, &factory)
                .audited_rotate_passphrase(
                    &before_bootstrap,
                    &Zeroizing::new(b"wrong current passphrase".to_vec()),
                    &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                    rotation_policy(),
                    rotation_randomness(0x73),
                    600,
                    audited_access_randomness(0x77),
                    &local,
                    &bootstrap,
                )
                .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
        assert_eq!(exact_bootstrap(&bootstrap), before_bootstrap);

        let session = active_session(locator, &local, &bootstrap, &factory);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::PassphraseRotate,
                AuditOutcomeV1::Failed,
                None,
                None
            )
        );
    }

    #[test]
    fn a_host_failure_between_the_two_prompts_is_audited_without_rotating() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        active_session(locator, &local, &bootstrap, &factory)
            .activate_audit_epoch(500, audited_access_randomness(0x81), &local)
            .unwrap();
        let before_bootstrap = exact_bootstrap(&bootstrap);

        active_session(locator, &local, &bootstrap, &factory)
            .record_audited_passphrase_rotation_host_failure(
                600,
                audited_access_randomness(0x83),
                &local,
            )
            .unwrap();

        assert_eq!(exact_bootstrap(&bootstrap), before_bootstrap);
        let session = active_session(locator, &local, &bootstrap, &factory);
        assert_eq!(
            latest_audit_facts(&session),
            (
                AuditActionV1::PassphraseRotate,
                AuditOutcomeV1::Failed,
                None,
                None
            )
        );
    }

    #[test]
    fn a_pre_audit_vault_refuses_the_audited_rotation_boundary() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        let before_bootstrap = exact_bootstrap(&bootstrap);
        assert_eq!(
            active_session(locator, &local, &bootstrap, &factory)
                .audited_rotate_passphrase(
                    &before_bootstrap,
                    &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                    &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                    rotation_policy(),
                    rotation_randomness(0x87),
                    600,
                    audited_access_randomness(0x89),
                    &local,
                    &bootstrap,
                )
                .err(),
            Some(ApplicationError::InvalidInput)
        );
        assert_eq!(exact_bootstrap(&bootstrap), before_bootstrap);
    }

    #[test]
    fn rotation_inputs_are_bounded_and_redacted() {
        assert_eq!(
            PassphraseRotationPolicyV1::new(1, 1, 1).err(),
            Some(ApplicationError::InvalidInput)
        );
        let policy = rotation_policy();
        assert_eq!(policy.memory_kib(), 8 * 1024);
        assert_eq!(policy.iterations(), 1);
        assert_eq!(policy.lanes(), 1);
        assert_eq!(
            format!("{policy:?}"),
            "PassphraseRotationPolicyV1 { memory_kib: 8192, iterations: 1, lanes: 1 }"
        );
        let randomness = rotation_randomness(0x91);
        assert_eq!(
            format!("{randomness:?}"),
            "PassphraseRotationRandomnessV1(<redacted>)"
        );
    }

    #[test]
    fn a_rotation_journal_round_trips_and_refuses_a_non_successor() {
        let (_locator, local, bootstrap, _factory, _backend, _) = vault_with_one_item();
        let before_bootstrap = exact_bootstrap(&bootstrap);
        let active = durable_active(&local);

        // Generation zero is not a successor of itself, and arbitrary bytes are
        // not a bootstrap.
        assert_eq!(
            PendingRotationV1::new(active.clone(), before_bootstrap.clone()).err(),
            Some(ApplicationError::IntegrityFailure)
        );
        assert_eq!(
            PendingRotationV1::new(active.clone(), b"not a bootstrap".to_vec()).err(),
            Some(ApplicationError::IntegrityFailure)
        );

        let prepared = prepared_rotation(&local, &bootstrap, 0x97);
        let journal = PendingRotationV1::new(active, prepared.bootstrap().to_vec()).unwrap();
        let encoded = LocalVaultStateV1::PendingRotation(journal.clone())
            .encode()
            .unwrap();
        assert_eq!(
            LocalVaultStateV1::decode(&encoded).unwrap(),
            LocalVaultStateV1::PendingRotation(journal.clone())
        );
        assert_eq!(
            format!("{:?}", LocalVaultStateV1::PendingRotation(journal.clone())),
            "LocalVaultStateV1::PendingRotation"
        );
        assert!(format!("{journal:?}").starts_with("PendingRotationV1 { bootstrap_bytes: "));
        assert!(format!("{prepared:?}").starts_with("PreparedPassphraseRotationV1 {"));
        assert_ne!(
            journal.next_bootstrap_id().unwrap(),
            journal.superseded_bootstrap_id()
        );
        assert_eq!(
            journal.intended_active().unwrap().bootstrap_id(),
            journal.next_bootstrap_id().unwrap()
        );
        assert_eq!(
            journal.active().bootstrap_id(),
            journal.superseded_bootstrap_id()
        );
    }

    /// A rotation may raise the KDF cost and may never lower it.
    ///
    /// The fixture vault is created at the V1 floor, so this raises it first
    /// and then tries to come back down: a rotation run specifically to improve
    /// a person's security must not be the thing that weakens it.
    #[test]
    fn a_rotation_refuses_to_weaken_the_key_derivation() {
        let (locator, local, bootstrap, factory, _backend, _) = vault_with_one_item();
        active_session(locator, &local, &bootstrap, &factory)
            .rotate_passphrase(
                &exact_bootstrap(&bootstrap),
                &Zeroizing::new(OLD_PASSPHRASE.to_vec()),
                &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                PassphraseRotationPolicyV1::new(16 * 1024, 2, 1).unwrap(),
                rotation_randomness(0xa1),
                &local,
                &bootstrap,
            )
            .unwrap();
        let raised = exact_bootstrap(&bootstrap);
        assert_eq!(
            BootstrapV1::decode(&raised).unwrap().kdf.memory_kib,
            16 * 1024
        );

        for weaker in [
            PassphraseRotationPolicyV1::new(8 * 1024, 2, 1).unwrap(),
            PassphraseRotationPolicyV1::new(16 * 1024, 1, 1).unwrap(),
        ] {
            let session = open_active_vault(
                Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .unwrap();
            assert_eq!(
                session
                    .rotate_passphrase(
                        &raised,
                        &Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                        &Zeroizing::new(b"a third passphrase".to_vec()),
                        weaker,
                        rotation_randomness(0xa7),
                        &local,
                        &bootstrap,
                    )
                    .err(),
                Some(ApplicationError::InvalidInput)
            );
            assert_eq!(exact_bootstrap(&bootstrap), raised);
        }
    }

    /// A bootstrap store that refuses to install anything.
    ///
    /// It answers `load_latest` truthfully, which is what lets the roll-back
    /// prove the install never happened.
    struct RefusingInstallStore<'store>(&'store MemoryBootstrapStore);

    impl BootstrapStore for RefusingInstallStore<'_> {
        fn load_latest(
            &self,
            locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
            self.0.load_latest(locator)
        }

        fn put_generation(
            &self,
            _locator: BootstrapLocator,
            _expected_previous: Option<BootstrapId>,
            _exact_bootstrap: &[u8],
        ) -> Result<(), BootstrapStoreError> {
            Err(BootstrapStoreError::Conflict)
        }

        fn supersede_generation(
            &self,
            _locator: BootstrapLocator,
            _superseded: BootstrapId,
        ) -> Result<(), BootstrapStoreError> {
            panic!("a rotation that could not install must never retire anything")
        }
    }

    /// A journalled rotation the provider refused has an exit.
    ///
    /// Rolling forward is the rule, but it is unanswerable only while it is
    /// unknown how far the ceremony got. When the store still serves the
    /// generation this rotation meant to retire, the install provably did not
    /// happen and neither did the retirement that strictly follows it — so the
    /// vault is exactly where it started and standing still is safe. Without
    /// this the vault would be wedged forever by a transient provider answer.
    #[test]
    fn a_rotation_the_provider_refused_leaves_the_old_passphrase_working() {
        let (locator, local, bootstrap, factory, _backend, item_id) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0xb1);
        let before_bootstrap = exact_bootstrap(&bootstrap);

        let refusing = RefusingInstallStore(&bootstrap);
        assert_eq!(
            crate::recover_pending_rotation(locator, &local, &refusing).err(),
            Some(ApplicationError::IntegrityFailure)
        );

        // The journal is gone, the durable world is the one the rotation
        // started from, and the old passphrase still opens the vault.
        assert_eq!(exact_bootstrap(&bootstrap), before_bootstrap);
        assert_eq!(
            crate::VaultAccessV1::locked(locator)
                .status(&local)
                .unwrap()
                .state(),
            crate::VaultStatusStateV1::Locked
        );
        let reopened = open_active_vault(
            Zeroizing::new(OLD_PASSPHRASE.to_vec()),
            locator,
            &local,
            &bootstrap,
            &factory,
        )
        .unwrap();
        assert!(reopened.get_item(item_id).unwrap().is_some());
        assert_eq!(
            open_active_vault(
                Zeroizing::new(NEW_PASSPHRASE.to_vec()),
                locator,
                &local,
                &bootstrap,
                &factory,
            )
            .err(),
            Some(ApplicationError::AuthenticationFailed)
        );
    }

    /// A store that refuses to install *and* cannot be read.
    ///
    /// Nothing can be proved about how far the ceremony got, so the journal
    /// must stay exactly where it is.
    struct BlindRefusingStore;

    impl BootstrapStore for BlindRefusingStore {
        fn load_latest(
            &self,
            _locator: BootstrapLocator,
        ) -> Result<Option<Vec<u8>>, BootstrapStoreError> {
            Err(BootstrapStoreError::Unavailable)
        }

        fn put_generation(
            &self,
            _locator: BootstrapLocator,
            _expected_previous: Option<BootstrapId>,
            _exact_bootstrap: &[u8],
        ) -> Result<(), BootstrapStoreError> {
            Err(BootstrapStoreError::Unavailable)
        }

        fn supersede_generation(
            &self,
            _locator: BootstrapLocator,
            _superseded: BootstrapId,
        ) -> Result<(), BootstrapStoreError> {
            unreachable!()
        }
    }

    #[test]
    fn an_unprovable_rotation_failure_leaves_the_journal_alone() {
        let (locator, local, bootstrap, _factory, _backend, _) = vault_with_one_item();
        install_pending_rotation(&local, &bootstrap, 0xb7);
        let exact_pending = local.0.lock().unwrap().clone().unwrap();

        assert_eq!(
            crate::recover_pending_rotation(locator, &local, &BlindRefusingStore).err(),
            Some(ApplicationError::StorageUnavailable)
        );
        assert_eq!(local.0.lock().unwrap().clone().unwrap(), exact_pending);
        assert_eq!(
            crate::VaultAccessV1::locked(locator)
                .status(&local)
                .unwrap()
                .state(),
            crate::VaultStatusStateV1::RecoveryRequired
        );
    }

    #[test]
    fn a_generation_that_is_still_live_cannot_be_superseded() {
        let (locator, local, bootstrap, _factory, _backend, _) = vault_with_one_item();
        let live = BootstrapV1::decode(&exact_bootstrap(&bootstrap))
            .unwrap()
            .id()
            .unwrap();
        assert_eq!(
            bootstrap.supersede_generation(locator, live),
            Err(BootstrapStoreError::Conflict)
        );
        assert!(local.0.lock().unwrap().is_some());
    }
}
