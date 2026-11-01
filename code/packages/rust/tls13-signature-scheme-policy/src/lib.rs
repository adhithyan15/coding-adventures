//! # Data-driven TLS 1.3 signature-scheme policy
//!
//! RFC 8446 assigns separate wire identifiers to each signature algorithm,
//! hash, and public-key encoding combination. Keeping those values together in
//! immutable descriptors prevents a later handshake layer from pairing one
//! wire identifier with another scheme's hash, MGF, salt, or key policy.
//!
//! This crate recognizes only the three RSASSA-PSS RSAE schemes. It deliberately
//! excludes EdDSA pending repository hardening, ECDSA pending repository-owned
//! verification, and RSASSA-PSS public-key OIDs pending separate key parsing.
//! Selection is policy metadata only and does not prove an implementation is
//! present or suitable for production.

#![forbid(unsafe_code)]

use core::fmt;

/// The number of signature schemes recognized by this package.
pub const SUPPORTED_SIGNATURE_SCHEME_COUNT: usize = 3;

/// Hash algorithms bound by the recognized TLS signature schemes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SignatureHash {
    /// SHA-256 with a 32-byte output.
    Sha256,
    /// SHA-384 with a 48-byte output.
    Sha384,
    /// SHA-512 with a 64-byte output.
    Sha512,
}

impl SignatureHash {
    /// Return the digest output length in bytes.
    #[must_use]
    pub const fn output_length(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }
}

/// Public-key encodings required by recognized schemes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PublicKeyAlgorithm {
    /// X.509 `SubjectPublicKeyInfo` using the `rsaEncryption` OID.
    RsaEncryption,
}

/// The repository-targeted RFC 8446 RSASSA-PSS RSAE schemes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SignatureScheme {
    /// `rsa_pss_rsae_sha256` (`0x0804`).
    RsaPssRsaeSha256,
    /// `rsa_pss_rsae_sha384` (`0x0805`).
    RsaPssRsaeSha384,
    /// `rsa_pss_rsae_sha512` (`0x0806`).
    RsaPssRsaeSha512,
}

impl SignatureScheme {
    /// Decode one exact recognized TLS signature-scheme wire identifier.
    pub const fn from_wire_id(wire_id: u16) -> Result<Self, PolicyError> {
        match wire_id {
            0x0804 => Ok(Self::RsaPssRsaeSha256),
            0x0805 => Ok(Self::RsaPssRsaeSha384),
            0x0806 => Ok(Self::RsaPssRsaeSha512),
            _ => Err(PolicyError::UnknownSignatureScheme),
        }
    }

    /// Return this scheme's two-byte TLS wire identifier.
    #[must_use]
    pub const fn wire_id(self) -> u16 {
        match self {
            Self::RsaPssRsaeSha256 => 0x0804,
            Self::RsaPssRsaeSha384 => 0x0805,
            Self::RsaPssRsaeSha512 => 0x0806,
        }
    }

    /// Return the immutable RFC-bound parameters for this scheme.
    #[must_use]
    pub const fn parameters(self) -> SignatureSchemeParameters {
        let hash = match self {
            Self::RsaPssRsaeSha256 => SignatureHash::Sha256,
            Self::RsaPssRsaeSha384 => SignatureHash::Sha384,
            Self::RsaPssRsaeSha512 => SignatureHash::Sha512,
        };
        SignatureSchemeParameters {
            scheme: self,
            message_hash: hash,
            mgf1_hash: hash,
            public_key_algorithm: PublicKeyAlgorithm::RsaEncryption,
            salt_length: hash.output_length() as u8,
        }
    }
}

/// Immutable parameters bound to one RFC 8446 signature scheme.
///
/// Private fields prevent callers from associating a standard wire identifier
/// with a different hash, mask-generation hash, salt length, or key form.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignatureSchemeParameters {
    scheme: SignatureScheme,
    message_hash: SignatureHash,
    mgf1_hash: SignatureHash,
    public_key_algorithm: PublicKeyAlgorithm,
    salt_length: u8,
}

impl SignatureSchemeParameters {
    /// Return the scheme to which every remaining field is bound.
    #[must_use]
    pub const fn scheme(self) -> SignatureScheme {
        self.scheme
    }

    /// Return the hash applied to the complete TLS signature input.
    #[must_use]
    pub const fn message_hash(self) -> SignatureHash {
        self.message_hash
    }

    /// Return the hash used by MGF1.
    #[must_use]
    pub const fn mgf1_hash(self) -> SignatureHash {
        self.mgf1_hash
    }

    /// Return the required certificate public-key algorithm.
    #[must_use]
    pub const fn public_key_algorithm(self) -> PublicKeyAlgorithm {
        self.public_key_algorithm
    }

    /// Return the exact RFC-required RSA-PSS salt length in bytes.
    #[must_use]
    pub const fn salt_length(self) -> usize {
        self.salt_length as usize
    }
}

/// Stable, payload-free signature-scheme policy failures.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PolicyError {
    /// The wire identifier is not one of the recognized RSAE schemes.
    UnknownSignatureScheme,
    /// The caller attempted to construct an empty allowlist.
    EmptyPolicy,
    /// The caller repeated one scheme in the ordered allowlist.
    DuplicateSignatureScheme,
    /// A recognized scheme was not present in the caller's allowlist.
    DisallowedSignatureScheme,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnknownSignatureScheme => "unknown TLS 1.3 signature scheme",
            Self::EmptyPolicy => "empty TLS 1.3 signature-scheme policy",
            Self::DuplicateSignatureScheme => "duplicate TLS 1.3 signature scheme",
            Self::DisallowedSignatureScheme => "TLS 1.3 signature scheme is not allowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PolicyError {}

/// One ordered caller-owned signature-scheme allowlist.
///
/// The policy borrows rather than copies or allocates its entries. The order is
/// retained for a later extension encoder; this crate neither encodes nor sends
/// an offer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureSchemePolicy<'a> {
    allowed: &'a [SignatureScheme],
}

impl<'a> SignatureSchemePolicy<'a> {
    /// Validate a non-empty, duplicate-free ordered allowlist.
    pub fn new(allowed: &'a [SignatureScheme]) -> Result<Self, PolicyError> {
        if allowed.is_empty() {
            return Err(PolicyError::EmptyPolicy);
        }
        for (index, scheme) in allowed.iter().enumerate() {
            if allowed[..index].contains(scheme) {
                return Err(PolicyError::DuplicateSignatureScheme);
            }
        }
        Ok(Self { allowed })
    }

    /// Return the exact caller preference order for later syntax composition.
    #[must_use]
    pub const fn ordered_schemes(self) -> &'a [SignatureScheme] {
        self.allowed
    }

    /// Authorize one exact recognized wire identifier under this policy.
    ///
    /// The result proves only policy membership. It does not prove that a
    /// verifier exists, that a peer used the scheme, or that a signature is
    /// valid.
    pub fn select(self, wire_id: u16) -> Result<AuthorizedSignatureScheme, PolicyError> {
        let scheme = SignatureScheme::from_wire_id(wire_id)?;
        if !self.allowed.contains(&scheme) {
            return Err(PolicyError::DisallowedSignatureScheme);
        }
        Ok(AuthorizedSignatureScheme { scheme })
    }
}

/// Policy-only authorization for one recognized scheme.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct AuthorizedSignatureScheme {
    scheme: SignatureScheme,
}

impl AuthorizedSignatureScheme {
    /// Return the authorized scheme.
    #[must_use]
    pub const fn scheme(self) -> SignatureScheme {
        self.scheme
    }

    /// Return the immutable RFC parameters bound to the scheme.
    #[must_use]
    pub const fn parameters(self) -> SignatureSchemeParameters {
        self.scheme.parameters()
    }
}
