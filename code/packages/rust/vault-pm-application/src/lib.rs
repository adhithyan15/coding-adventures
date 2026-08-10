//! VLT-PM05 host-neutral password-manager application core.
//!
//! This first implementation slice owns canonical application persistence and
//! encrypted object framing. Hosts retain storage, clock, entropy, process,
//! environment, and credential-custody authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod codec;
mod crypto;
mod disclosure;
mod initialize;
mod lifecycle;
mod mutation;
mod open;
mod repository;
mod search;
mod state;
mod status;
mod verifier;

pub use codec::{
    decode_device_certificate, decode_item_revision, decode_signed_commit,
    encode_device_certificate, encode_item_revision, encode_signed_commit, CatalogV1,
    LocalSecretV1,
};
pub use crypto::{
    open_local_secret, open_object, seal_local_secret, seal_object, LocalSecretRandomness,
    ObjectKind, ObjectRandomness, V1Keys,
};
pub use disclosure::{
    RevealedSecretEncodingV1, RevealedSecretV1, SecretDisclosureIntentV1, SecretFieldV1,
};
pub use initialize::{
    complete_generation_zero, prepare_generation_zero, rehydrate_prepared_init,
    GenerationZeroPolicyV1, GenerationZeroRandomness, PreparedGenerationZero,
    GENERATION_ZERO_RANDOM_BYTES,
};
pub use lifecycle::{LockedVaultV1, VaultAccessV1};
pub use mutation::{
    AddItemRandomnessV1, DeleteItemRandomnessV1, ReplaceItemRandomnessV1,
    ResolveItemConflictRandomnessV1, RestoreItemRandomnessV1, ADD_ITEM_RANDOM_BYTES,
    DELETE_ITEM_RANDOM_BYTES, REPLACE_ITEM_RANDOM_BYTES, RESOLVE_ITEM_CONFLICT_RANDOM_BYTES,
    RESTORE_ITEM_RANDOM_BYTES,
};
pub use open::{
    open_active_vault, recover_pending_publication, ItemHistoryViewV1, UnlockedVaultV1,
    DEFAULT_ITEM_HISTORY_LIMIT, MAX_ITEM_HISTORY_LIMIT,
};
pub use repository::{
    ApplicationRepository, ApplicationRepositoryError, ApplicationRepositoryFactory,
    V1ApplicationRepositoryFactory,
};
pub use state::{
    ActiveStateV1, AuthorityFingerprint, BootstrapLocator, BootstrapStore, BootstrapStoreError,
    LocalStateStore, LocalStateStoreError, LocalVaultStateV1, PreparedInitV1, PublicationJournalV1,
};
pub use status::{VaultStatusStateV1, VaultStatusV1};
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
