//! Fixed-memory TLS 1.3 traffic-secret update derivation.
//!
//! RFC 8446 section 7.2 advances an application traffic secret with exactly
//! this operation:
//!
//! ```text
//! application_traffic_secret_N+1 =
//!     HKDF-Expand-Label(application_traffic_secret_N,
//!                       "traffic upd", "", Hash.length)
//! ```
//!
//! This crate fixes the label and empty context inside one small composition
//! boundary. The current secret's [`HkdfPrk`] also fixes SHA-256 or SHA-384 and
//! therefore the only accepted caller-owned output length. The crate does not
//! decide when an update is authorized or install the result into record state.

#![forbid(unsafe_code)]

use coding_adventures_hkdf::HkdfPrk;
use core::fmt;
use tls13_hkdf_label::{hkdf_expand_label_fixed, HkdfExpandLabelError};

/// The RFC 8446 section 7.2 label, kept private so callers cannot reinterpret
/// the update operation.
const TRAFFIC_UPDATE_LABEL: &[u8] = b"traffic upd";

/// RFC 8446 fixes the update context to an empty byte string.
const EMPTY_CONTEXT: &[u8] = b"";

/// Stable, secret-free failures for one traffic-secret update.
#[derive(Debug, PartialEq, Eq)]
pub enum TrafficSecretUpdateError {
    /// Caller-owned output was not exactly the selected hash's output length.
    OutputLengthMismatch,
    /// The repository-owned fixed-memory expansion failed.
    Derivation(HkdfExpandLabelError),
}

impl fmt::Display for TrafficSecretUpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputLengthMismatch => formatter.write_str(
                "TLS 1.3 updated traffic-secret output must match the selected hash length",
            ),
            Self::Derivation(error) => {
                write!(formatter, "TLS 1.3 traffic-secret update failed: {error}")
            }
        }
    }
}

impl std::error::Error for TrafficSecretUpdateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OutputLengthMismatch => None,
            Self::Derivation(error) => Some(error),
        }
    }
}

/// Derive the traffic secret immediately following `current_secret`.
///
/// The hash algorithm and exact output length travel together in
/// `current_secret`. This function rejects a shorter or longer output before
/// changing it, then invokes repository-owned fixed-memory
/// `HKDF-Expand-Label` with the protocol-fixed `"traffic upd"` label and empty
/// context. The current secret is borrowed and remains unchanged. Successful
/// secret bytes stay in caller-owned output storage and never cross the ABI
/// return slot by value.
///
/// Callers should place `output` in wipe-on-drop storage, import the successful
/// exact prefix into a fresh [`HkdfPrk`], and atomically install that owner with
/// fresh record sequence state. This function deliberately performs none of
/// those state transitions.
///
/// # Example
///
/// ```
/// use coding_adventures_hkdf::{FixedHkdfHash, HkdfPrk};
/// use coding_adventures_zeroize::Zeroizing;
/// use tls13_traffic_secret_update::derive_next_traffic_secret;
///
/// let current = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &[0x42; 32])?;
/// let mut next = Zeroizing::new([0u8; 32]);
/// derive_next_traffic_secret(&current, &mut next[..])?;
/// let next_owner = HkdfPrk::import_exact(FixedHkdfHash::Sha256, &next[..])?;
/// assert_eq!(next_owner.algorithm(), FixedHkdfHash::Sha256);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn derive_next_traffic_secret(
    current_secret: &HkdfPrk,
    output: &mut [u8],
) -> Result<(), TrafficSecretUpdateError> {
    if output.len() != current_secret.algorithm().hash_len() {
        return Err(TrafficSecretUpdateError::OutputLengthMismatch);
    }

    hkdf_expand_label_fixed(current_secret, TRAFFIC_UPDATE_LABEL, EMPTY_CONTEXT, output)
        .map_err(TrafficSecretUpdateError::Derivation)
}
