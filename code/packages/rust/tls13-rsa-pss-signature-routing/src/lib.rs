//! # Policy-bound TLS 1.3 RSA-PSS signature routing
//!
//! TLS carries a signature-scheme identifier beside each `CertificateVerify`
//! signature. This crate requires that identifier to match an already
//! authorized scheme, then routes an exact typed digest, validated RSA key,
//! signature, scratch buffer, and fixed-profile limits to the corresponding
//! repository-owned SHA-256, SHA-384, or SHA-512 RSASSA-PSS verifier.
//!
//! The caller computes the digest. This crate does not build or hash the TLS
//! signed input, prove transcript provenance, parse certificates, establish
//! trust, sequence a handshake, process records, or perform I/O.

#![deny(unsafe_code)]

use core::fmt;
use rsa_pss_sha256_signature::{
    verify_rsa_pss_sha256_signature, RsaPssSha256SignatureErrorKind, RsaPssSha256SignatureLimits,
    VerifiedRsaPssSha256Signature,
};
use rsa_pss_sha384_signature::{
    verify_rsa_pss_sha384_signature, RsaPssSha384SignatureErrorKind, RsaPssSha384SignatureLimits,
    VerifiedRsaPssSha384Signature,
};
use rsa_pss_sha512_signature::{
    verify_rsa_pss_sha512_signature, RsaPssSha512SignatureErrorKind, RsaPssSha512SignatureLimits,
    VerifiedRsaPssSha512Signature,
};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_signature_scheme_policy::{AuthorizedSignatureScheme, SignatureScheme};
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// The fixed RSA-PSS profile selected by one authorized TLS scheme.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RsaPssSignatureProfile {
    /// SHA-256, MGF1-SHA-256, and a 32-byte salt.
    Sha256,
    /// SHA-384, MGF1-SHA-384, and a 48-byte salt.
    Sha384,
    /// SHA-512, MGF1-SHA-512, and a 64-byte salt.
    Sha512,
}

impl RsaPssSignatureProfile {
    const fn for_scheme(scheme: SignatureScheme) -> Self {
        match scheme {
            SignatureScheme::RsaPssRsaeSha256 => Self::Sha256,
            SignatureScheme::RsaPssRsaeSha384 => Self::Sha384,
            SignatureScheme::RsaPssRsaeSha512 => Self::Sha512,
        }
    }
}

/// One exact caller-computed message digest.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RsaPssMessageDigest<'digest> {
    /// A 32-byte SHA-256 digest.
    Sha256(&'digest [u8; 32]),
    /// A 48-byte SHA-384 digest.
    Sha384(&'digest [u8; 48]),
    /// A 64-byte SHA-512 digest.
    Sha512(&'digest [u8; 64]),
}

impl<'digest> RsaPssMessageDigest<'digest> {
    /// Return the fixed profile implied by the digest length.
    #[must_use]
    pub const fn profile(self) -> RsaPssSignatureProfile {
        match self {
            Self::Sha256(_) => RsaPssSignatureProfile::Sha256,
            Self::Sha384(_) => RsaPssSignatureProfile::Sha384,
            Self::Sha512(_) => RsaPssSignatureProfile::Sha512,
        }
    }

    /// Borrow the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(self) -> &'digest [u8] {
        match self {
            Self::Sha256(digest) => digest,
            Self::Sha384(digest) => digest,
            Self::Sha512(digest) => digest,
        }
    }
}

impl fmt::Debug for RsaPssMessageDigest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RsaPssMessageDigest")
            .field(&self.profile())
            .finish()
    }
}

/// One fixed verifier's already-validated public work limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13RsaPssSignatureLimits {
    Sha256(RsaPssSha256SignatureLimits),
    Sha384(RsaPssSha384SignatureLimits),
    Sha512(RsaPssSha512SignatureLimits),
}

impl Tls13RsaPssSignatureLimits {
    /// Return the fixed profile to which these limits belong.
    #[must_use]
    pub const fn profile(self) -> RsaPssSignatureProfile {
        match self {
            Self::Sha256(_) => RsaPssSignatureProfile::Sha256,
            Self::Sha384(_) => RsaPssSignatureProfile::Sha384,
            Self::Sha512(_) => RsaPssSignatureProfile::Sha512,
        }
    }
}

/// Stable failures retaining no signature, digest, key, or caller lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13RsaPssSignatureRoutingErrorKind {
    /// The decoded message did not carry the authorized scheme's wire ID.
    SignatureSchemeMismatch,
    /// The caller supplied a digest for another fixed profile.
    DigestProfileMismatch,
    /// The caller supplied limits for another fixed profile.
    LimitsProfileMismatch,
    /// SHA-256 fixed verification failed.
    Sha256(RsaPssSha256SignatureErrorKind),
    /// SHA-384 fixed verification failed.
    Sha384(RsaPssSha384SignatureErrorKind),
    /// SHA-512 fixed verification failed.
    Sha512(RsaPssSha512SignatureErrorKind),
}

/// A redacted routing or fixed-verifier failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13RsaPssSignatureRoutingError {
    kind: Tls13RsaPssSignatureRoutingErrorKind,
}

impl Tls13RsaPssSignatureRoutingError {
    const fn new(kind: Tls13RsaPssSignatureRoutingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> Tls13RsaPssSignatureRoutingErrorKind {
        self.kind
    }
}

impl fmt::Debug for Tls13RsaPssSignatureRoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13RsaPssSignatureRoutingError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Tls13RsaPssSignatureRoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS RSA-PSS signature routing error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for Tls13RsaPssSignatureRoutingError {}

macro_rules! routed_signature {
    ($name:ident, $fixed:ident, $debug_name:literal) => {
        /// Non-copyable evidence from one policy-authorized fixed verifier.
        pub struct $name<'message, 'body, 'key, 'digest> {
            authorized_scheme: AuthorizedSignatureScheme,
            message: &'message CertificateVerifyMessage<'body>,
            digest: RsaPssMessageDigest<'digest>,
            signature: $fixed<'key, 'message, 'digest>,
        }

        impl<'message, 'body, 'key, 'digest> $name<'message, 'body, 'key, 'digest> {
            /// Return the exact policy-authorized scheme used for routing.
            #[must_use]
            pub const fn authorized_scheme(&self) -> AuthorizedSignatureScheme {
                self.authorized_scheme
            }

            /// Return the exact structurally decoded message that supplied the signature.
            #[must_use]
            pub const fn message(&self) -> &'message CertificateVerifyMessage<'body> {
                self.message
            }

            /// Return the exact caller-computed digest used by the fixed verifier.
            #[must_use]
            pub const fn digest(&self) -> RsaPssMessageDigest<'digest> {
                self.digest
            }

            /// Return the selected fixed verifier's evidence.
            #[must_use]
            pub const fn signature(&self) -> &$fixed<'key, 'message, 'digest> {
                &self.signature
            }
        }

        impl fmt::Debug for $name<'_, '_, '_, '_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str($debug_name)
            }
        }
    };
}

routed_signature!(
    VerifiedTls13RsaPssSha256Signature,
    VerifiedRsaPssSha256Signature,
    "VerifiedTls13RsaPssSha256Signature"
);
routed_signature!(
    VerifiedTls13RsaPssSha384Signature,
    VerifiedRsaPssSha384Signature,
    "VerifiedTls13RsaPssSha384Signature"
);
routed_signature!(
    VerifiedTls13RsaPssSha512Signature,
    VerifiedRsaPssSha512Signature,
    "VerifiedTls13RsaPssSha512Signature"
);

/// Non-copyable policy-bound evidence from one fixed RSA-PSS verifier.
pub enum VerifiedTls13RsaPssSignature<'message, 'body, 'key, 'digest> {
    Sha256(VerifiedTls13RsaPssSha256Signature<'message, 'body, 'key, 'digest>),
    Sha384(VerifiedTls13RsaPssSha384Signature<'message, 'body, 'key, 'digest>),
    Sha512(VerifiedTls13RsaPssSha512Signature<'message, 'body, 'key, 'digest>),
}

impl<'message, 'body, 'key, 'digest> VerifiedTls13RsaPssSignature<'message, 'body, 'key, 'digest> {
    /// Return the selected fixed profile.
    #[must_use]
    pub const fn profile(&self) -> RsaPssSignatureProfile {
        match self {
            Self::Sha256(_) => RsaPssSignatureProfile::Sha256,
            Self::Sha384(_) => RsaPssSignatureProfile::Sha384,
            Self::Sha512(_) => RsaPssSignatureProfile::Sha512,
        }
    }

    /// Return the exact policy-authorized scheme used for routing.
    #[must_use]
    pub const fn authorized_scheme(&self) -> AuthorizedSignatureScheme {
        match self {
            Self::Sha256(verified) => verified.authorized_scheme(),
            Self::Sha384(verified) => verified.authorized_scheme(),
            Self::Sha512(verified) => verified.authorized_scheme(),
        }
    }

    /// Return the exact structurally decoded message that supplied the signature.
    #[must_use]
    pub const fn message(&self) -> &'message CertificateVerifyMessage<'body> {
        match self {
            Self::Sha256(verified) => verified.message(),
            Self::Sha384(verified) => verified.message(),
            Self::Sha512(verified) => verified.message(),
        }
    }

    /// Return the exact validated RSA key used by the fixed verifier.
    #[must_use]
    pub const fn public_key(&self) -> ValidatedRsaPublicKey<'key> {
        match self {
            Self::Sha256(verified) => verified.signature().public_key(),
            Self::Sha384(verified) => verified.signature().public_key(),
            Self::Sha512(verified) => verified.signature().public_key(),
        }
    }

    /// Return the exact caller-computed digest used by the fixed verifier.
    #[must_use]
    pub const fn digest(&self) -> RsaPssMessageDigest<'digest> {
        match self {
            Self::Sha256(verified) => verified.digest(),
            Self::Sha384(verified) => verified.digest(),
            Self::Sha512(verified) => verified.digest(),
        }
    }
}

impl fmt::Debug for VerifiedTls13RsaPssSignature<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sha256(_) => "VerifiedTls13RsaPssSignature::Sha256",
            Self::Sha384(_) => "VerifiedTls13RsaPssSignature::Sha384",
            Self::Sha512(_) => "VerifiedTls13RsaPssSignature::Sha512",
        })
    }
}

/// Route one authorized TLS signature to its exact fixed RSA-PSS verifier.
///
/// The message wire ID, typed digest, and typed limits are checked before the
/// selected fixed verifier performs RSA work or can mutate DB scratch.
pub fn verify_tls13_rsa_pss_signature<'message, 'body, 'key, 'digest>(
    authorized_scheme: AuthorizedSignatureScheme,
    message: &'message CertificateVerifyMessage<'body>,
    public_key: ValidatedRsaPublicKey<'key>,
    digest: RsaPssMessageDigest<'digest>,
    data_block_scratch: &mut [u8],
    limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedTls13RsaPssSignature<'message, 'body, 'key, 'digest>,
    Tls13RsaPssSignatureRoutingError,
> {
    let scheme = authorized_scheme.scheme();
    if message.algorithm_wire_id() != scheme.wire_id() {
        return Err(Tls13RsaPssSignatureRoutingError::new(
            Tls13RsaPssSignatureRoutingErrorKind::SignatureSchemeMismatch,
        ));
    }

    let profile = RsaPssSignatureProfile::for_scheme(scheme);
    if digest.profile() != profile {
        return Err(Tls13RsaPssSignatureRoutingError::new(
            Tls13RsaPssSignatureRoutingErrorKind::DigestProfileMismatch,
        ));
    }
    if limits.profile() != profile {
        return Err(Tls13RsaPssSignatureRoutingError::new(
            Tls13RsaPssSignatureRoutingErrorKind::LimitsProfileMismatch,
        ));
    }

    match (scheme, digest, limits) {
        (
            SignatureScheme::RsaPssRsaeSha256,
            RsaPssMessageDigest::Sha256(digest),
            Tls13RsaPssSignatureLimits::Sha256(limits),
        ) => {
            let signature = verify_rsa_pss_sha256_signature(
                public_key,
                message.signature(),
                digest,
                data_block_scratch,
                limits,
            )
            .map_err(|error| {
                Tls13RsaPssSignatureRoutingError::new(Tls13RsaPssSignatureRoutingErrorKind::Sha256(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedTls13RsaPssSignature::Sha256(
                VerifiedTls13RsaPssSha256Signature {
                    authorized_scheme,
                    message,
                    digest: RsaPssMessageDigest::Sha256(digest),
                    signature,
                },
            ))
        }
        (
            SignatureScheme::RsaPssRsaeSha384,
            RsaPssMessageDigest::Sha384(digest),
            Tls13RsaPssSignatureLimits::Sha384(limits),
        ) => {
            let signature = verify_rsa_pss_sha384_signature(
                public_key,
                message.signature(),
                digest,
                data_block_scratch,
                limits,
            )
            .map_err(|error| {
                Tls13RsaPssSignatureRoutingError::new(Tls13RsaPssSignatureRoutingErrorKind::Sha384(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedTls13RsaPssSignature::Sha384(
                VerifiedTls13RsaPssSha384Signature {
                    authorized_scheme,
                    message,
                    digest: RsaPssMessageDigest::Sha384(digest),
                    signature,
                },
            ))
        }
        (
            SignatureScheme::RsaPssRsaeSha512,
            RsaPssMessageDigest::Sha512(digest),
            Tls13RsaPssSignatureLimits::Sha512(limits),
        ) => {
            let signature = verify_rsa_pss_sha512_signature(
                public_key,
                message.signature(),
                digest,
                data_block_scratch,
                limits,
            )
            .map_err(|error| {
                Tls13RsaPssSignatureRoutingError::new(Tls13RsaPssSignatureRoutingErrorKind::Sha512(
                    error.kind(),
                ))
            })?;
            Ok(VerifiedTls13RsaPssSignature::Sha512(
                VerifiedTls13RsaPssSha512Signature {
                    authorized_scheme,
                    message,
                    digest: RsaPssMessageDigest::Sha512(digest),
                    signature,
                },
            ))
        }
        _ => unreachable!("profile equality proves a matching fixed verifier route"),
    }
}
