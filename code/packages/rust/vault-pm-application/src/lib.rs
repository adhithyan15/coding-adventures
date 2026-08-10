//! VLT-PM05 host-neutral password-manager application core.
//!
//! This first implementation slice owns canonical application persistence and
//! encrypted object framing. Hosts retain storage, clock, entropy, process,
//! environment, and credential-custody authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod codec;
mod crypto;
mod initialize;
mod repository;
mod state;
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
pub use initialize::{
    prepare_generation_zero, GenerationZeroPolicyV1, GenerationZeroRandomness,
    PreparedGenerationZero, GENERATION_ZERO_RANDOM_BYTES,
};
pub use repository::{
    ApplicationRepository, ApplicationRepositoryError, ApplicationRepositoryFactory,
    V1ApplicationRepositoryFactory,
};
pub use state::{
    ActiveStateV1, AuthorityFingerprint, BootstrapLocator, BootstrapStore, BootstrapStoreError,
    LocalStateStore, LocalStateStoreError, LocalVaultStateV1, PreparedInitV1, PublicationJournalV1,
};
pub use verifier::V1SingleDeviceVerifier;

use core::fmt::{self, Debug, Display, Formatter};

/// Closed, payload-free application failure taxonomy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationError {
    /// No initialized owner state exists for the requested locator.
    NotInitialized,
    /// Owner state already exists and cannot be replaced by initialization.
    AlreadyInitialized,
    /// Caller input violates a V1 precondition.
    InvalidInput,
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
    /// An infallible internal relation was violated.
    InternalInvariant,
}

impl ApplicationError {
    fn label(self) -> &'static str {
        match self {
            Self::NotInitialized => "NotInitialized",
            Self::AlreadyInitialized => "AlreadyInitialized",
            Self::InvalidInput => "InvalidInput",
            Self::BoundExceeded => "BoundExceeded",
            Self::ConcurrentHost => "ConcurrentHost",
            Self::StorageUnavailable => "StorageUnavailable",
            Self::IntegrityFailure => "IntegrityFailure",
            Self::Unsupported => "Unsupported",
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
