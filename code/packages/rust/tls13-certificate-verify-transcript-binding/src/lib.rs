//! # Transcript-bound TLS 1.3 `CertificateVerify` input
//!
//! This crate snapshots one immutable, cipher-suite-selected repository TLS
//! transcript, maps its bound hash algorithm into the exact RFC 8446
//! `CertificateVerify` framing metadata, and constructs the role-specific
//! signed input. A second stage hashes only that retained input through the
//! existing authorized RSA-PSS scheme composition. A third stage accepts only
//! that complete outer evidence and delegates the exact decoded message,
//! validated RSA key, scratch, and fixed limits to repository-owned RSA-PSS
//! verification. A consuming form performs those stages internally and admits
//! the exact authenticated CertificateVerify body to an owned transcript as
//! one complete handshake encoding.
//!
//! The API accepts neither detached transcript bytes nor caller-selected
//! transcript-hash metadata. It does not prove the preceding ordered handshake
//! state, authorize the selected role, bind a certificate key to a reviewed
//! path, authenticate Finished, or make TLS or HTTPS available.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_certificate_verify_input::{
    build_certificate_verify_input, CertificateVerifyInput, CertificateVerifyRole, InputError,
    TranscriptHashAlgorithm,
};
use tls13_certificate_verify_message::{CertificateVerifyMessage, MessageError};
use tls13_cipher_suite_policy::HkdfHash;
use tls13_handshake::{
    encode_tls13_handshake, HandshakeErrorKind, Tls13HandshakePolicy, Tls13HandshakeType,
};
use tls13_rsa_pss_certificate_verify::{
    hash_tls13_certificate_verify_input, verify_tls13_rsa_pss_certificate_verify,
    HashedTls13CertificateVerifyInput, Tls13RsaPssCertificateVerifyError,
    Tls13RsaPssCertificateVerifyErrorKind, VerifiedTls13RsaPssCertificateVerify,
};
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use tls13_signature_scheme_policy::AuthorizedSignatureScheme;
use tls13_transcript::{SelectedTls13Transcript, TranscriptErrorKind, TranscriptHash};
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

/// Stable, payload-free failures from consuming CertificateVerify transcript
/// advancement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateVerifyTranscriptAdvanceErrorKind {
    /// The exact borrowed CertificateVerify body was not structurally valid.
    MessageFraming(MessageError),
    /// The selected transcript could not be framed into the signed input.
    SignedInput(InputError),
    /// Repository-owned RSA-PSS verification rejected the exact message.
    SignatureVerification(Tls13RsaPssCertificateVerifyErrorKind),
    /// The authenticated body could not be framed as one handshake message.
    HandshakeFraming(HandshakeErrorKind),
    /// The complete authenticated encoding could not advance the transcript.
    Transcript(TranscriptErrorKind),
}

/// One redacted consuming CertificateVerify transcript-advance failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateVerifyTranscriptAdvanceError {
    kind: CertificateVerifyTranscriptAdvanceErrorKind,
}

impl CertificateVerifyTranscriptAdvanceError {
    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> CertificateVerifyTranscriptAdvanceErrorKind {
        self.kind
    }
}

impl fmt::Display for CertificateVerifyTranscriptAdvanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 CertificateVerify transcript-advance error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for CertificateVerifyTranscriptAdvanceError {}

/// Non-copyable evidence owning one transcript advanced by an authenticated
/// exact CertificateVerify handshake encoding.
pub struct OwnedVerifiedCertificateVerifyTranscript<'body, 'key> {
    transcript: SelectedTls13Transcript,
    before_snapshot: TranscriptHash,
    after_snapshot: TranscriptHash,
    role: CertificateVerifyRole,
    authorized_scheme: AuthorizedSignatureScheme,
    certificate_verify_body: &'body [u8],
    public_key: ValidatedRsaPublicKey<'key>,
    encoded_handshake_length: usize,
}

impl<'body, 'key> OwnedVerifiedCertificateVerifyTranscript<'body, 'key> {
    /// Borrow the exact selected transcript after authenticated advancement.
    #[must_use]
    pub const fn transcript(&self) -> &SelectedTls13Transcript {
        &self.transcript
    }

    /// Return the transcript snapshot authenticated by CertificateVerify.
    #[must_use]
    pub const fn before_snapshot(&self) -> TranscriptHash {
        self.before_snapshot
    }

    /// Return the transcript snapshot after hashing the complete message.
    #[must_use]
    pub const fn after_snapshot(&self) -> TranscriptHash {
        self.after_snapshot
    }

    /// Return the endpoint role framed into the authenticated signed input.
    #[must_use]
    pub const fn role(&self) -> CertificateVerifyRole {
        self.role
    }

    /// Return the exact policy-authorized signature scheme.
    #[must_use]
    pub const fn authorized_scheme(&self) -> AuthorizedSignatureScheme {
        self.authorized_scheme
    }

    /// Borrow the exact structurally decoded body hashed into the transcript.
    #[must_use]
    pub const fn certificate_verify_body(&self) -> &'body [u8] {
        self.certificate_verify_body
    }

    /// Return the exact validated RSA key used for signature verification.
    #[must_use]
    pub const fn public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.public_key
    }

    /// Return the complete handshake encoding length added to the transcript.
    #[must_use]
    pub const fn encoded_handshake_length(&self) -> usize {
        self.encoded_handshake_length
    }
}

impl fmt::Debug for OwnedVerifiedCertificateVerifyTranscript<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OwnedVerifiedCertificateVerifyTranscript")
    }
}

/// Authenticate one exact CertificateVerify body and advance an owned selected
/// transcript with that complete handshake encoding.
///
/// The exact body is decoded internally before any signed-input output changes.
/// Signature verification is completed against the transcript's captured
/// pre-message snapshot before the body is framed and admitted to the owned
/// transcript. Every failure consumes the transcript and returns only a stable
/// redacted category, preventing caller retry or reuse of partial state.
#[allow(clippy::too_many_arguments)]
pub fn verify_and_advance_owned_certificate_verify_transcript<'body, 'key>(
    mut transcript: SelectedTls13Transcript,
    role: CertificateVerifyRole,
    authorized_scheme: AuthorizedSignatureScheme,
    certificate_verify_body: &'body [u8],
    public_key: ValidatedRsaPublicKey<'key>,
    signed_input_output: &mut [u8],
    handshake_output: &mut [u8],
    data_block_scratch: &mut [u8],
    limits: Tls13RsaPssSignatureLimits,
) -> Result<
    OwnedVerifiedCertificateVerifyTranscript<'body, 'key>,
    CertificateVerifyTranscriptAdvanceError,
> {
    let message = CertificateVerifyMessage::decode(certificate_verify_body).map_err(|error| {
        CertificateVerifyTranscriptAdvanceError {
            kind: CertificateVerifyTranscriptAdvanceErrorKind::MessageFraming(error),
        }
    })?;
    let before_snapshot = transcript.snapshot();

    {
        let bound = bind_certificate_verify_transcript(&transcript, role, signed_input_output)
            .map_err(|error| CertificateVerifyTranscriptAdvanceError {
                kind: CertificateVerifyTranscriptAdvanceErrorKind::SignedInput(error),
            })?;
        let hashed = hash_transcript_bound_certificate_verify_input(authorized_scheme, &bound);
        verify_transcript_bound_certificate_verify(
            &hashed,
            &message,
            public_key,
            data_block_scratch,
            limits,
        )
        .map_err(|error| CertificateVerifyTranscriptAdvanceError {
            kind: CertificateVerifyTranscriptAdvanceErrorKind::SignatureVerification(error.kind()),
        })?;
    }

    let handshake_policy = Tls13HandshakePolicy::new(
        &[Tls13HandshakeType::CertificateVerify],
        certificate_verify_body.len(),
    )
    .map_err(|error| CertificateVerifyTranscriptAdvanceError {
        kind: CertificateVerifyTranscriptAdvanceErrorKind::HandshakeFraming(error.kind()),
    })?;
    let encoded_handshake_length = encode_tls13_handshake(
        &handshake_policy,
        Tls13HandshakeType::CertificateVerify,
        certificate_verify_body,
        handshake_output,
    )
    .map_err(|error| CertificateVerifyTranscriptAdvanceError {
        kind: CertificateVerifyTranscriptAdvanceErrorKind::HandshakeFraming(error.kind()),
    })?;
    transcript
        .update(&handshake_output[..encoded_handshake_length])
        .map_err(|error| CertificateVerifyTranscriptAdvanceError {
            kind: CertificateVerifyTranscriptAdvanceErrorKind::Transcript(error.kind()),
        })?;
    let after_snapshot = transcript.snapshot();

    Ok(OwnedVerifiedCertificateVerifyTranscript {
        transcript,
        before_snapshot,
        after_snapshot,
        role,
        authorized_scheme,
        certificate_verify_body,
        public_key,
        encoded_handshake_length,
    })
}
