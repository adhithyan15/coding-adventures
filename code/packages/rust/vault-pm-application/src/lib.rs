//! VLT-PM05 host-neutral password-manager application core.
//!
//! This first implementation slice owns canonical application persistence and
//! encrypted object framing. Hosts retain storage, clock, entropy, process,
//! environment, and credential-custody authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod access;
mod attachment;
mod audit;
mod codec;
mod crypto;
mod disclosure;
mod doctor;
mod export;
mod initialize;
mod lifecycle;
mod mutation;
mod open;
mod repository;
mod restore;
mod rotate;
mod search;
mod state;
mod status;
mod totp;
mod verifier;

pub use access::AuditedAccessResultV1;
pub use attachment::{
    attachment_name_from_path, AttachmentContentV1, AttachmentSummaryV1, ATTACHMENT_CHUNK_BYTES,
    ATTACHMENT_DEK_BYTES, MAX_ATTACHMENT_BYTES, MAX_ATTACHMENT_CHUNKS, MAX_ATTACHMENT_NAME_BYTES,
};
pub use audit::{
    AuditEventViewV1, AuditVerificationV1, DEFAULT_AUDIT_HISTORY_LIMIT, MAX_AUDIT_HISTORY_LIMIT,
};
pub use codec::{
    decode_attachment_chunk, decode_device_certificate, decode_item_revision,
    decode_signed_audit_event, decode_signed_commit, encode_attachment_chunk,
    encode_device_certificate, encode_item_revision, encode_signed_audit_event,
    encode_signed_commit, AttachmentManifestV1, CatalogV1, LocalSecretV1,
};
pub use crypto::{
    open_local_secret, open_object, seal_local_secret, seal_object, LocalSecretRandomness,
    ObjectKind, ObjectRandomness, V1Keys,
};
pub use disclosure::{
    RevealedSecretEncodingV1, RevealedSecretV1, SecretDisclosureIntentV1, SecretFieldV1,
};
pub use doctor::{VaultDoctorReportV1, VaultDoctorStateV1};
pub use export::{
    open_portable_with_passphrase, OpenedPortableSnapshotV1, PortableExportArtifactV1,
    PortableExportCompletenessV1, PortableExportOutcomeV1, PortableExportPolicyV1,
    PortableExportRandomnessV1, PortableOpenPolicyV1, MAX_PORTABLE_EXPORT_ARTIFACT_BYTES,
    MAX_PORTABLE_EXPORT_PASSPHRASE_BYTES, MAX_PORTABLE_EXPORT_PLAINTEXT_BYTES,
    PORTABLE_EXPORT_RANDOM_BYTES,
};
pub use initialize::{
    complete_generation_zero, prepare_audited_generation_zero, prepare_generation_zero,
    rehydrate_prepared_init, AuditedGenerationZeroRandomness, GenerationZeroPolicyV1,
    GenerationZeroRandomness, PreparedGenerationZero, AUDITED_GENERATION_ZERO_RANDOM_BYTES,
    GENERATION_ZERO_RANDOM_BYTES,
};
pub use lifecycle::{LockedVaultV1, UnlockRecoveryV1, VaultAccessV1};
pub use mutation::{
    attachment_random_bytes, portable_import_random_bytes, AddItemRandomnessV1,
    AttachmentRandomnessV1, AuditedAccessRandomnessV1, DeleteItemRandomnessV1,
    PortableImportRandomnessV1, ReplaceItemRandomnessV1, ResolveItemConflictRandomnessV1,
    RestoreItemRandomnessV1, ADD_ITEM_RANDOM_BYTES, AUDITED_ACCESS_RANDOM_BYTES,
    DELETE_ITEM_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES, RESOLVE_ITEM_CONFLICT_RANDOM_BYTES,
    RESTORE_ITEM_RANDOM_BYTES,
};
pub use open::{
    open_active_vault, recover_pending_publication, ApiKeyConflictMergeInputV1,
    ApiKeyConflictMergePreparationV1, AuditedApiKeyConflictMergePreparationV1,
    AuditedCardConflictMergePreparationV1, AuditedDatabaseCredentialConflictMergePreparationV1,
    AuditedLoginConflictMergePreparationV1, AuditedLoginEditPreparationV1,
    AuditedOpaqueConflictMergePreparationV1, AuditedSecureNoteConflictMergePreparationV1,
    AuditedTotpConflictMergePreparationV1, CardConflictMergeInputV1,
    CardConflictMergePreparationV1, DatabaseCredentialConflictMergeInputV1,
    DatabaseCredentialConflictMergePreparationV1, ItemHistoryViewV1,
    LoginConflictMergePreparationV1, LoginEditInputV1, LoginEditPreparationV1,
    OpaqueConflictMergeInputV1, OpaqueConflictMergePreparationV1, SecureNoteConflictMergeInputV1,
    SecureNoteConflictMergePreparationV1, TotpConflictMergeInputV1, TotpConflictMergePreparationV1,
    UnlockedVaultV1, DEFAULT_ITEM_HISTORY_LIMIT, MAX_ITEM_HISTORY_LIMIT,
};
pub use repository::{
    ApplicationRepository, ApplicationRepositoryError, ApplicationRepositoryFactory,
    V1ApplicationRepositoryFactory,
};
pub use restore::{PortableRestoreExpectationV1, PortableRestoreVerificationV1};
pub use rotate::{
    commit_passphrase_rotation, prepare_passphrase_rotation, recover_pending_rotation,
    PassphraseRotationPolicyV1, PassphraseRotationRandomnessV1, PreparedPassphraseRotationV1,
    PASSPHRASE_ROTATION_RANDOM_BYTES,
};
pub use state::{
    ActiveStateV1, AuthorityFingerprint, BootstrapLocator, BootstrapStore, BootstrapStoreError,
    LocalStateStore, LocalStateStoreError, LocalVaultStateV1, PendingRotationV1, PreparedInitV1,
    PublicationJournalV1,
};
pub use status::{VaultStatusStateV1, VaultStatusV1};
pub use totp::TotpCodeV1;
pub use verifier::V1SingleDeviceVerifier;

use core::fmt::{self, Debug, Display, Formatter};

/// Closed, payload-free application failure taxonomy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationError {
    /// No initialized owner state exists for the requested locator.
    NotInitialized,
    /// Owner state already exists and cannot be replaced by initialization.
    AlreadyInitialized,
    /// An operation requires a live authenticated vault session.
    Locked,
    /// Passphrase authentication or its indistinguishable root-wrap check failed.
    AuthenticationFailed,
    /// Caller input violates a V1 precondition.
    InvalidInput,
    /// The requested item or revision does not exist in reachable vault state.
    NotFound,
    /// A fixed parser or collection bound would be exceeded.
    BoundExceeded,
    /// A host compare-exchange lost to another local writer.
    ConcurrentHost,
    /// An injected host store is unavailable without exposing provider detail.
    StorageUnavailable,
    /// A persisted value is malformed, unauthenticated, or cross-vault.
    IntegrityFailure,
    /// A requested version, suite, or object kind is not supported.
    Unsupported,
    /// One item has multiple current candidates and needs explicit resolution.
    ConflictRequired,
    /// An infallible internal relation was violated.
    InternalInvariant,
}

impl ApplicationError {
    fn label(self) -> &'static str {
        match self {
            Self::NotInitialized => "NotInitialized",
            Self::AlreadyInitialized => "AlreadyInitialized",
            Self::Locked => "Locked",
            Self::AuthenticationFailed => "AuthenticationFailed",
            Self::InvalidInput => "InvalidInput",
            Self::NotFound => "NotFound",
            Self::BoundExceeded => "BoundExceeded",
            Self::ConcurrentHost => "ConcurrentHost",
            Self::StorageUnavailable => "StorageUnavailable",
            Self::IntegrityFailure => "IntegrityFailure",
            Self::Unsupported => "Unsupported",
            Self::ConflictRequired => "ConflictRequired",
            Self::InternalInvariant => "InternalInvariant",
        }
    }
}

impl Debug for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl Display for ApplicationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("vault-pm-application: ")?;
        formatter.write_str(self.label())
    }
}

impl std::error::Error for ApplicationError {}
