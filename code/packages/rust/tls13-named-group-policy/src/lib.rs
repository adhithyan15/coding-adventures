//! Data-driven TLS 1.3 named-group policy and immutable parameters.
//!
//! A named group binds one exact wire identifier to a key-agreement routing
//! identifier and the standard key-exchange and shared-secret lengths. Keeping
//! those values together prevents later protocol layers from independently
//! reinterpreting a selected group.
//!
//! This crate deliberately stops at policy metadata. It does not parse a key
//! share, generate a secret, invoke or approve a key-agreement implementation,
//! validate a peer public key, derive a shared secret, process a handshake,
//! perform I/O, or provide TLS or HTTPS.

#![forbid(unsafe_code)]

use core::fmt;

/// Number of named groups recognized by this package.
pub const SUPPORTED_NAMED_GROUP_COUNT: usize = 2;

/// Repository-targeted key-agreement algorithm identifiers.
///
/// These are routing metadata. Possessing one does not prove that an
/// implementation exists or is suitable for production use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum KeyAgreementAlgorithm {
    /// ECDH over the NIST P-256 curve.
    EcdhSecp256r1,
    /// X25519 scalar multiplication as used by TLS named group `0x001d`.
    X25519,
}

/// Named groups with reviewed standard parameter descriptors.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NamedGroup {
    /// ECDH over secp256r1 (`0x0017`).
    Secp256r1,
    /// X25519 (`0x001d`).
    X25519,
}

impl NamedGroup {
    /// Decode one exact recognized named-group wire identifier.
    pub const fn from_wire_id(wire_id: u16) -> Result<Self, PolicyError> {
        match wire_id {
            0x0017 => Ok(Self::Secp256r1),
            0x001d => Ok(Self::X25519),
            _ => Err(PolicyError::UnknownNamedGroup),
        }
    }

    /// Return this group's two-byte TLS wire identifier.
    #[must_use]
    pub const fn wire_id(self) -> u16 {
        match self {
            Self::Secp256r1 => 0x0017,
            Self::X25519 => 0x001d,
        }
    }

    /// Return the immutable standard parameters bound to this group.
    #[must_use]
    pub const fn parameters(self) -> NamedGroupParameters {
        match self {
            Self::Secp256r1 => {
                NamedGroupParameters::new(self, KeyAgreementAlgorithm::EcdhSecp256r1, 65, 32)
            }
            Self::X25519 => NamedGroupParameters::new(self, KeyAgreementAlgorithm::X25519, 32, 32),
        }
    }

    const fn policy_index(self) -> usize {
        match self {
            Self::Secp256r1 => 0,
            Self::X25519 => 1,
        }
    }
}

/// Immutable RFC-bound parameters for one recognized group.
///
/// Fields are private so callers cannot associate a standard wire identifier
/// with different algorithm or length metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NamedGroupParameters {
    group: NamedGroup,
    key_agreement: KeyAgreementAlgorithm,
    key_exchange_length: u16,
    shared_secret_length: u16,
}

impl NamedGroupParameters {
    const fn new(
        group: NamedGroup,
        key_agreement: KeyAgreementAlgorithm,
        key_exchange_length: u16,
        shared_secret_length: u16,
    ) -> Self {
        Self {
            group,
            key_agreement,
            key_exchange_length,
            shared_secret_length,
        }
    }

    /// Return the group to which every remaining field is bound.
    #[must_use]
    pub const fn group(self) -> NamedGroup {
        self.group
    }

    /// Return the key-agreement routing identifier.
    #[must_use]
    pub const fn key_agreement(self) -> KeyAgreementAlgorithm {
        self.key_agreement
    }

    /// Return the exact ClientHello and ServerHello key-exchange length.
    #[must_use]
    pub const fn key_exchange_length(self) -> usize {
        self.key_exchange_length as usize
    }

    /// Return the exact derived shared-secret length.
    #[must_use]
    pub const fn shared_secret_length(self) -> usize {
        self.shared_secret_length as usize
    }
}

/// Stable named-group policy failures.
///
/// Errors retain neither wire identifiers nor observed lengths.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PolicyError {
    /// The wire identifier has no reviewed parameter descriptor.
    UnknownNamedGroup,
    /// The caller attempted to construct a policy with no groups.
    EmptyPolicy,
    /// The caller repeated a group in one allowlist.
    DuplicateNamedGroup,
    /// A known group was not present in the caller's allowlist.
    DisallowedNamedGroup,
    /// A key-exchange value did not match the selected group's exact length.
    InvalidKeyExchangeLength,
    /// A derived shared secret did not match the selected group's exact length.
    InvalidSharedSecretLength,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownNamedGroup => "unknown TLS named group",
            Self::EmptyPolicy => "empty TLS named-group policy",
            Self::DuplicateNamedGroup => "duplicate TLS named group",
            Self::DisallowedNamedGroup => "TLS named group is not allowed",
            Self::InvalidKeyExchangeLength => "invalid TLS named-group key-exchange length",
            Self::InvalidSharedSecretLength => "invalid TLS named-group shared-secret length",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyError {}

/// Allocation-free allowlist for recognized named groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedGroupPolicy {
    allowed: [bool; SUPPORTED_NAMED_GROUP_COUNT],
}

impl NamedGroupPolicy {
    /// Validate a non-empty, duplicate-free group allowlist.
    pub fn new(groups: &[NamedGroup]) -> Result<Self, PolicyError> {
        if groups.is_empty() {
            return Err(PolicyError::EmptyPolicy);
        }

        let mut allowed = [false; SUPPORTED_NAMED_GROUP_COUNT];
        for group in groups.iter().copied() {
            let index = group.policy_index();
            if allowed[index] {
                return Err(PolicyError::DuplicateNamedGroup);
            }
            allowed[index] = true;
        }
        Ok(Self { allowed })
    }

    /// Select one exact wire identifier already authorized by this policy.
    pub fn select(&self, wire_id: u16) -> Result<SelectedNamedGroup, PolicyError> {
        let group = NamedGroup::from_wire_id(wire_id)?;
        if !self.allowed[group.policy_index()] {
            return Err(PolicyError::DisallowedNamedGroup);
        }
        Ok(SelectedNamedGroup {
            parameters: group.parameters(),
        })
    }

    /// Return whether one typed group is in this policy.
    #[must_use]
    pub const fn allows(&self, group: NamedGroup) -> bool {
        self.allowed[group.policy_index()]
    }
}

/// One policy-authorized group with all routing and length metadata bound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SelectedNamedGroup {
    parameters: NamedGroupParameters,
}

impl SelectedNamedGroup {
    /// Return the immutable standard descriptor.
    #[must_use]
    pub const fn parameters(self) -> NamedGroupParameters {
        self.parameters
    }

    /// Validate the exact public key-exchange length for this selected group.
    pub const fn validate_key_exchange_length(self, length: usize) -> Result<(), PolicyError> {
        if length != self.parameters.key_exchange_length() {
            return Err(PolicyError::InvalidKeyExchangeLength);
        }
        Ok(())
    }

    /// Validate the exact derived shared-secret length for this selected group.
    pub const fn validate_shared_secret_length(self, length: usize) -> Result<(), PolicyError> {
        if length != self.parameters.shared_secret_length() {
            return Err(PolicyError::InvalidSharedSecretLength);
        }
        Ok(())
    }
}
