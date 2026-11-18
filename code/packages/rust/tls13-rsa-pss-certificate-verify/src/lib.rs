//! # Bound TLS 1.3 RSA-PSS `CertificateVerify`
//!
//! This crate hashes one exact, already-framed TLS 1.3 `CertificateVerify`
//! signed input with the hash bound to an authorized RSA-PSS RSAE scheme. A
//! second stage accepts only that retained hash evidence and delegates the
//! exact decoded signature, validated RSA key, scratch, and fixed limits to
//! repository-owned RSA-PSS routing.
//!
//! It does not prove transcript provenance, select a cipher suite, parse or
//! validate certificates, sequence a handshake, protect records, or perform
//! I/O.

#![deny(unsafe_code)]

use coding_adventures_sha256::sha256;
use coding_adventures_sha384::sha384;
use coding_adventures_sha512::sum512;
use core::fmt;
use tls13_certificate_verify_input::CertificateVerifyInput;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_rsa_pss_signature_routing::{
    verify_tls13_rsa_pss_signature, RsaPssMessageDigest, Tls13RsaPssSignatureLimits,
    Tls13RsaPssSignatureRoutingErrorKind, VerifiedTls13RsaPssSignature,
};
use tls13_signature_scheme_policy::{AuthorizedSignatureScheme, SignatureScheme};
use x509_rsa_public_key::ValidatedRsaPublicKey;

enum OwnedRsaPssMessageDigest {
    Sha256([u8; 32]),
    Sha384([u8; 48]),
    Sha512([u8; 64]),
}

/// Non-copyable evidence binding one authorized scheme to the digest of one
/// exact framed `CertificateVerify` signed input.
pub struct HashedTls13CertificateVerifyInput<'input, 'bytes> {
    authorized_scheme: AuthorizedSignatureScheme,
    input: &'input CertificateVerifyInput<'bytes>,
    digest: OwnedRsaPssMessageDigest,
}

impl<'input, 'bytes> HashedTls13CertificateVerifyInput<'input, 'bytes> {
    /// Return the exact policy-authorized scheme that selected the hash.
    #[must_use]
    pub const fn authorized_scheme(&self) -> AuthorizedSignatureScheme {
        self.authorized_scheme
    }

    /// Return the exact framed signed input that was hashed.
    #[must_use]
    pub const fn input(&self) -> &'input CertificateVerifyInput<'bytes> {
        self.input
    }

    /// Borrow the owned digest under its exact fixed RSA-PSS profile.
    #[must_use]
    pub fn digest<'digest>(&'digest self) -> RsaPssMessageDigest<'digest> {
        match &self.digest {
            OwnedRsaPssMessageDigest::Sha256(digest) => RsaPssMessageDigest::Sha256(digest),
            OwnedRsaPssMessageDigest::Sha384(digest) => RsaPssMessageDigest::Sha384(digest),
            OwnedRsaPssMessageDigest::Sha512(digest) => RsaPssMessageDigest::Sha512(digest),
        }
    }
}

impl fmt::Debug for HashedTls13CertificateVerifyInput<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HashedTls13CertificateVerifyInput")
    }
}

/// Hash one exact framed signed input with the authorized scheme's fixed hash.
///
/// All supported input encodings are far below the repository hash primitives'
/// message-length bounds, so this operation is infallible after the input and
/// scheme have independently been validated.
#[must_use]
pub fn hash_tls13_certificate_verify_input<'input, 'bytes>(
    authorized_scheme: AuthorizedSignatureScheme,
    input: &'input CertificateVerifyInput<'bytes>,
) -> HashedTls13CertificateVerifyInput<'input, 'bytes> {
    let digest = match authorized_scheme.scheme() {
        SignatureScheme::RsaPssRsaeSha256 => {
            OwnedRsaPssMessageDigest::Sha256(sha256(input.as_bytes()))
        }
        SignatureScheme::RsaPssRsaeSha384 => {
            OwnedRsaPssMessageDigest::Sha384(sha384(input.as_bytes()))
        }
        SignatureScheme::RsaPssRsaeSha512 => {
            OwnedRsaPssMessageDigest::Sha512(sum512(input.as_bytes()))
        }
    };
    HashedTls13CertificateVerifyInput {
        authorized_scheme,
        input,
        digest,
    }
}

/// Stable payload-free failures from fixed RSA-PSS signature routing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tls13RsaPssCertificateVerifyErrorKind {
    /// The retained hash evidence could not verify the exact message and key.
    SignatureRouting(Tls13RsaPssSignatureRoutingErrorKind),
}

/// One redacted bound `CertificateVerify` failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Tls13RsaPssCertificateVerifyError {
    kind: Tls13RsaPssCertificateVerifyErrorKind,
}

impl Tls13RsaPssCertificateVerifyError {
    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> Tls13RsaPssCertificateVerifyErrorKind {
        self.kind
    }
}

impl fmt::Debug for Tls13RsaPssCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tls13RsaPssCertificateVerifyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Tls13RsaPssCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS RSA-PSS CertificateVerify error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for Tls13RsaPssCertificateVerifyError {}

/// Non-copyable evidence binding the exact hashed input to exact routed RSA-PSS
/// verification evidence.
pub struct VerifiedTls13RsaPssCertificateVerify<
    'hashed,
    'input,
    'input_bytes,
    'message,
    'body,
    'key,
> {
    hashed_input: &'hashed HashedTls13CertificateVerifyInput<'input, 'input_bytes>,
    signature: VerifiedTls13RsaPssSignature<'message, 'body, 'key, 'hashed>,
}

impl<'hashed, 'input, 'input_bytes, 'message, 'body, 'key>
    VerifiedTls13RsaPssCertificateVerify<'hashed, 'input, 'input_bytes, 'message, 'body, 'key>
{
    /// Return the exact retained input-hash evidence.
    #[must_use]
    pub const fn hashed_input(
        &self,
    ) -> &'hashed HashedTls13CertificateVerifyInput<'input, 'input_bytes> {
        self.hashed_input
    }

    /// Return the exact fixed RSA-PSS routing evidence.
    #[must_use]
    pub const fn signature(&self) -> &VerifiedTls13RsaPssSignature<'message, 'body, 'key, 'hashed> {
        &self.signature
    }
}

impl fmt::Debug for VerifiedTls13RsaPssCertificateVerify<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedTls13RsaPssCertificateVerify")
    }
}

/// Verify one decoded RSA-PSS `CertificateVerify` using only exact retained
/// input-hash evidence.
pub fn verify_tls13_rsa_pss_certificate_verify<
    'hashed,
    'input,
    'input_bytes,
    'message,
    'body,
    'key,
>(
    hashed_input: &'hashed HashedTls13CertificateVerifyInput<'input, 'input_bytes>,
    message: &'message CertificateVerifyMessage<'body>,
    public_key: ValidatedRsaPublicKey<'key>,
    data_block_scratch: &mut [u8],
    limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedTls13RsaPssCertificateVerify<'hashed, 'input, 'input_bytes, 'message, 'body, 'key>,
    Tls13RsaPssCertificateVerifyError,
> {
    let signature = verify_tls13_rsa_pss_signature(
        hashed_input.authorized_scheme(),
        message,
        public_key,
        hashed_input.digest(),
        data_block_scratch,
        limits,
    )
    .map_err(|error| Tls13RsaPssCertificateVerifyError {
        kind: Tls13RsaPssCertificateVerifyErrorKind::SignatureRouting(error.kind()),
    })?;

    Ok(VerifiedTls13RsaPssCertificateVerify {
        hashed_input,
        signature,
    })
}
