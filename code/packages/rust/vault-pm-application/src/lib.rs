//! VLT-PM05 host-neutral password-manager application core.
//!
//! This first implementation slice owns canonical application persistence and
//! encrypted object framing. Hosts retain storage, clock, entropy, process,
//! environment, and credential-custody authority.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod codec;
mod crypto;
mod verifier;

pub use codec::{
    decode_device_certificate, decode_item_revision, decode_signed_commit,
    encode_device_certificate, encode_item_revision, encode_signed_commit, CatalogV1,
    LocalSecretV1,
};
pub use crypto::{open_object, seal_object, ObjectKind, ObjectRandomness, V1Keys};
pub use verifier::V1SingleDeviceVerifier;

use core::fmt::{self, Debug, Display, Formatter};

/// Closed, payload-free application failure taxonomy.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationError {
    /// Caller input violates a V1 precondition.
    InvalidInput,
    /// A fixed parser or collection bound would be exceeded.
    BoundExceeded,
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
            Self::InvalidInput => "InvalidInput",
            Self::BoundExceeded => "BoundExceeded",
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
