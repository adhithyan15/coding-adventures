//! # Transcript-bound TLS 1.3 `CertificateVerify` input
//!
//! This crate snapshots one immutable, cipher-suite-selected repository TLS
//! transcript, maps its bound hash algorithm into the exact RFC 8446
//! `CertificateVerify` framing metadata, and constructs the role-specific
//! signed input. A second stage hashes only that retained input through the
//! existing authorized RSA-PSS scheme composition. A third stage accepts only
//! that complete outer evidence and delegates the exact decoded message,
//! validated RSA key, scratch, and fixed limits to repository-owned RSA-PSS
//! verification.
//!
//! The API accepts neither detached transcript bytes nor caller-selected
//! transcript-hash metadata. It does not prove ordered handshake state,
//! authorize the selected role, bind a certificate key, or make TLS or HTTPS
//! available.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_certificate_verify_input::{
    build_certificate_verify_input, CertificateVerifyInput, CertificateVerifyRole, InputError,
    TranscriptHashAlgorithm,
};
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_cipher_suite_policy::HkdfHash;
use tls13_rsa_pss_certificate_verify::{
    hash_tls13_certificate_verify_input, verify_tls13_rsa_pss_certificate_verify,
    HashedTls13CertificateVerifyInput, Tls13RsaPssCertificateVerifyError,
    VerifiedTls13RsaPssCertificateVerify,
};
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use tls13_signature_scheme_policy::AuthorizedSignatureScheme;
use tls13_transcript::{SelectedTls13Transcript, TranscriptHash};
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// One exact framed input bound to the selected transcript state that produced
/// its snapshot.
///
/// This value intentionally derives neither `Copy` nor `Clone`. Its immutable
/// transcript borrow prevents that state from advancing while the evidence is
/// live, and its input borrow prevents caller-owned output mutation.
pub struct TranscriptBoundCertificateVerifyInput<'transcript, 'output> {
    transcript: &'transcript SelectedTls13Transcript,
    snapshot: TranscriptHash,
    input: CertificateVerifyInput<'output>,
}

impl<'transcript, 'output> TranscriptBoundCertificateVerifyInput<'transcript, 'output> {
    /// Return the exact selected transcript state retained by this evidence.
    #[must_use]
    pub const fn transcript(&self) -> &'transcript SelectedTls13Transcript {
        self.transcript
    }

    /// Return the exact suite-bound snapshot framed into the signed input.
    #[must_use]
    pub const fn snapshot(&self) -> TranscriptHash {
        self.snapshot
    }

    /// Return the exact role-bound RFC 8446 signed input.
    #[must_use]
    pub const fn input(&self) -> &CertificateVerifyInput<'output> {
        &self.input
    }
}

impl fmt::Debug for TranscriptBoundCertificateVerifyInput<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TranscriptBoundCertificateVerifyInput")
    }
}

/// Snapshot one selected transcript and construct its exact role-specific
/// `CertificateVerify` signed input.
///
/// The transcript chooses both digest bytes and digest-length metadata. The
/// caller chooses only the endpoint role and output storage. The underlying
/// framing boundary preflights output capacity before changing `output`.
pub fn bind_certificate_verify_transcript<'transcript, 'output>(
    transcript: &'transcript SelectedTls13Transcript,
    role: CertificateVerifyRole,
    output: &'output mut [u8],
) -> Result<TranscriptBoundCertificateVerifyInput<'transcript, 'output>, InputError> {
    let snapshot = transcript.snapshot();
    let algorithm = match snapshot.algorithm() {
        HkdfHash::Sha256 => TranscriptHashAlgorithm::Sha256,
        HkdfHash::Sha384 => TranscriptHashAlgorithm::Sha384,
    };
    let input = build_certificate_verify_input(role, algorithm, snapshot.as_bytes(), output)?;

    Ok(TranscriptBoundCertificateVerifyInput {
        transcript,
        snapshot,
        input,
    })
}

/// Non-copyable evidence retaining both transcript-bound framing and the exact
/// scheme-bound hash evidence derived from it.
pub struct HashedTranscriptBoundCertificateVerifyInput<'bound, 'transcript, 'output> {
    bound_input: &'bound TranscriptBoundCertificateVerifyInput<'transcript, 'output>,
    hashed_input: HashedTls13CertificateVerifyInput<'bound, 'output>,
}

impl<'bound, 'transcript, 'output>
    HashedTranscriptBoundCertificateVerifyInput<'bound, 'transcript, 'output>
{
    /// Return the exact selected-transcript framing evidence.
    #[must_use]
    pub const fn bound_input(
        &self,
    ) -> &'bound TranscriptBoundCertificateVerifyInput<'transcript, 'output> {
        self.bound_input
    }

    /// Return the exact authorized-scheme hash evidence.
    #[must_use]
    pub const fn hashed_input(&self) -> &HashedTls13CertificateVerifyInput<'bound, 'output> {
        &self.hashed_input
    }
}

impl fmt::Debug for HashedTranscriptBoundCertificateVerifyInput<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("HashedTranscriptBoundCertificateVerifyInput")
    }
}

/// Hash only the exact input retained by selected-transcript framing evidence.
#[must_use]
pub fn hash_transcript_bound_certificate_verify_input<'bound, 'transcript, 'output>(
    authorized_scheme: AuthorizedSignatureScheme,
    bound_input: &'bound TranscriptBoundCertificateVerifyInput<'transcript, 'output>,
) -> HashedTranscriptBoundCertificateVerifyInput<'bound, 'transcript, 'output> {
    let hashed_input = hash_tls13_certificate_verify_input(authorized_scheme, bound_input.input());
    HashedTranscriptBoundCertificateVerifyInput {
        bound_input,
        hashed_input,
    }
}

/// Non-copyable signature-verdict evidence retaining the complete outer
/// selected-transcript proof rather than only its lower hash evidence.
pub struct VerifiedTranscriptBoundCertificateVerify<
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key,
> {
    hashed_input:
        &'hashed HashedTranscriptBoundCertificateVerifyInput<'bound, 'transcript, 'output>,
    verification:
        VerifiedTls13RsaPssCertificateVerify<'hashed, 'bound, 'output, 'message, 'body, 'key>,
}

impl<'hashed, 'bound, 'transcript, 'output, 'message, 'body, 'key>
    VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'key,
    >
{
    /// Return the exact outer transcript-bound hash evidence required for this
    /// verdict.
    #[must_use]
    pub const fn hashed_input(
        &self,
    ) -> &'hashed HashedTranscriptBoundCertificateVerifyInput<'bound, 'transcript, 'output> {
        self.hashed_input
    }

    /// Return the exact lower repository-owned RSA-PSS verification evidence.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTls13RsaPssCertificateVerify<'hashed, 'bound, 'output, 'message, 'body, 'key>
    {
        &self.verification
    }
}

impl fmt::Debug for VerifiedTranscriptBoundCertificateVerify<'_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedTranscriptBoundCertificateVerify")
    }
}

/// Verify one exact decoded RSA-PSS `CertificateVerify` message using only
/// outer transcript-bound hash evidence.
pub fn verify_transcript_bound_certificate_verify<
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key,
>(
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    public_key: ValidatedRsaPublicKey<'key>,
    data_block_scratch: &mut [u8],
    limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'key,
    >,
    Tls13RsaPssCertificateVerifyError,
> {
    let verification = verify_tls13_rsa_pss_certificate_verify(
        hashed_input.hashed_input(),
        message,
        public_key,
        data_block_scratch,
        limits,
    )?;

    Ok(VerifiedTranscriptBoundCertificateVerify {
        hashed_input,
        verification,
    })
}
