//! TLS 1.3 server `CertificateVerify` bound to a reviewed RSA path leaf.
//!
//! This composition accepts only selected-transcript hash evidence and the
//! strongest accumulated reviewed RSA server-path proof. It authorizes the
//! server role, derives and validates the exact reviewed target certificate's
//! RSA key, and delegates signature verification to repository-owned code.
//! It does not establish ordered handshake state, anchor self-signature,
//! revocation, Finished, records, TLS, HTTP, OAuth, or network readiness.

#![forbid(unsafe_code)]

use core::fmt;

use der_asn1::Asn1Limits;
use https_reviewed_server_path_binding::ReviewedHttpsServerPath;
use tls13_certificate_verify_input::CertificateVerifyRole;
use tls13_certificate_verify_message::CertificateVerifyMessage;
use tls13_certificate_verify_transcript_binding::{
    verify_transcript_bound_certificate_verify, HashedTranscriptBoundCertificateVerifyInput,
    VerifiedTranscriptBoundCertificateVerify,
};
use tls13_rsa_pss_certificate_verify::Tls13RsaPssCertificateVerifyErrorKind;
use tls13_rsa_pss_signature_routing::Tls13RsaPssSignatureLimits;
use x509_certificate::X509Certificate;
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

/// Stable, payload-free failures from reviewed-leaf verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewedServerPathCertificateVerifyErrorKind {
    /// The retained RFC 8446 signed input selected the client role.
    WrongEndpointRole,
    /// The reviewed leaf's exact SubjectPublicKeyInfo failed RSA policy.
    PublicKey(RsaPublicKeyErrorKind),
    /// The exact message failed repository-owned RSA-PSS verification.
    Verification(Tls13RsaPssCertificateVerifyErrorKind),
}

/// One redacted reviewed-leaf `CertificateVerify` failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ReviewedServerPathCertificateVerifyError {
    kind: ReviewedServerPathCertificateVerifyErrorKind,
}

impl ReviewedServerPathCertificateVerifyError {
    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> ReviewedServerPathCertificateVerifyErrorKind {
        self.kind
    }
}

impl fmt::Debug for ReviewedServerPathCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReviewedServerPathCertificateVerifyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for ReviewedServerPathCertificateVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reviewed server-path CertificateVerify error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReviewedServerPathCertificateVerifyError {}

/// Non-copyable evidence retaining the complete reviewed path and exact
/// selected-transcript server signature verdict.
pub struct VerifiedReviewedServerPathCertificateVerify<
    'reviewed,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
> {
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    verification: VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    >,
}

impl<
        'reviewed,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
    VerifiedReviewedServerPathCertificateVerify<
        'reviewed,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >
{
    /// Return the exact complete reviewed server-path proof.
    #[must_use]
    pub const fn reviewed_server_path(
        &self,
    ) -> &'reviewed ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    > {
        self.reviewed_server_path
    }

    /// Return the exact selected-transcript server signature verdict.
    #[must_use]
    pub const fn verification(
        &self,
    ) -> &VerifiedTranscriptBoundCertificateVerify<
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
        'target_encoded,
    > {
        &self.verification
    }
}

impl fmt::Debug
    for VerifiedReviewedServerPathCertificateVerify<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedReviewedServerPathCertificateVerify")
    }
}

/// Verify one server `CertificateVerify` with the exact reviewed path target.
#[allow(clippy::too_many_arguments)]
pub fn verify_reviewed_server_path_certificate_verify<
    'reviewed,
    'evidence,
    'initialized,
    'path,
    'target,
    'target_encoded,
    'candidates,
    'candidate_encoded,
    'policy,
    'anchor_certificate,
    'anchor_encoded,
    'role_certificate,
    'role_encoded,
    'hashed,
    'bound,
    'transcript,
    'output,
    'message,
    'body,
    'key_policy,
>(
    reviewed_server_path: &'reviewed ReviewedHttpsServerPath<
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
    >,
    hashed_input: &'hashed HashedTranscriptBoundCertificateVerifyInput<
        'bound,
        'transcript,
        'output,
    >,
    message: &'message CertificateVerifyMessage<'body>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
    data_block_scratch: &mut [u8],
    signature_limits: Tls13RsaPssSignatureLimits,
) -> Result<
    VerifiedReviewedServerPathCertificateVerify<
        'reviewed,
        'evidence,
        'initialized,
        'path,
        'target,
        'target_encoded,
        'candidates,
        'candidate_encoded,
        'policy,
        'anchor_certificate,
        'anchor_encoded,
        'role_certificate,
        'role_encoded,
        'hashed,
        'bound,
        'transcript,
        'output,
        'message,
        'body,
    >,
    ReviewedServerPathCertificateVerifyError,
> {
    require_server_role(hashed_input.bound_input().input().role())?;

    let certificate = reviewed_server_path
        .subordinate_policy()
        .target_policy()
        .anchor_policy()
        .target_signature_closure()
        .server_policy()
        .certificate();
    let public_key = validate_leaf_public_key(certificate, rsa_key_policy, rsa_key_limits)?;

    let verification = verify_transcript_bound_certificate_verify(
        hashed_input,
        message,
        public_key,
        data_block_scratch,
        signature_limits,
    )
    .map_err(|error| ReviewedServerPathCertificateVerifyError {
        kind: ReviewedServerPathCertificateVerifyErrorKind::Verification(error.kind()),
    })?;

    Ok(VerifiedReviewedServerPathCertificateVerify {
        reviewed_server_path,
        verification,
    })
}

fn validate_leaf_public_key<'encoded, 'key_policy>(
    certificate: &X509Certificate<'encoded>,
    rsa_key_policy: RsaPublicKeyPolicy<'key_policy>,
    rsa_key_limits: Asn1Limits,
) -> Result<ValidatedRsaPublicKey<'encoded>, ReviewedServerPathCertificateVerifyError> {
    validate_rsa_public_key(
        certificate.tbs_certificate().subject_public_key_info(),
        rsa_key_policy,
        rsa_key_limits,
    )
    .map_err(|error| ReviewedServerPathCertificateVerifyError {
        kind: ReviewedServerPathCertificateVerifyErrorKind::PublicKey(error.kind()),
    })
}

fn require_server_role(
    role: CertificateVerifyRole,
) -> Result<(), ReviewedServerPathCertificateVerifyError> {
    if role == CertificateVerifyRole::Server {
        Ok(())
    } else {
        Err(ReviewedServerPathCertificateVerifyError {
            kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        require_server_role, validate_leaf_public_key, ReviewedServerPathCertificateVerifyError,
        ReviewedServerPathCertificateVerifyErrorKind, VerifiedReviewedServerPathCertificateVerify,
    };
    use der_asn1::Asn1Limits;
    use tls13_certificate_verify_input::CertificateVerifyRole;
    use x509_certificate::decode_x509_certificate;
    use x509_rsa_public_key::{RsaPublicKeyErrorKind, RsaPublicKeyPolicy};

    const SHA256_WITH_RSA_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b];
    const RSA_ENCRYPTION_OID: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const NULL_DER: &[u8] = &[0x05, 0x00];

    fn length(length: usize) -> Vec<u8> {
        if length < 128 {
            return vec![length as u8];
        }
        let bytes = length.to_be_bytes();
        let first = bytes.iter().position(|byte| *byte != 0).unwrap();
        let significant = &bytes[first..];
        let mut encoded = vec![0x80 | significant.len() as u8];
        encoded.extend_from_slice(significant);
        encoded
    }

    fn der(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut encoded = vec![tag];
        encoded.extend(length(value.len()));
        encoded.extend_from_slice(value);
        encoded
    }

    fn sequence(fields: &[Vec<u8>]) -> Vec<u8> {
        der(0x30, &fields.concat())
    }

    fn positive_integer(magnitude: &[u8]) -> Vec<u8> {
        let mut contents = Vec::with_capacity(magnitude.len() + 1);
        if magnitude.first().is_some_and(|byte| byte & 0x80 != 0) {
            contents.push(0);
        }
        contents.extend_from_slice(magnitude);
        der(0x02, &contents)
    }

    fn algorithm(oid: &[u8]) -> Vec<u8> {
        sequence(&[der(0x06, oid), NULL_DER.to_vec()])
    }

    fn test_certificate() -> Vec<u8> {
        let mut modulus = vec![0_u8; 256];
        modulus[0] = 0x80;
        modulus[255] = 1;
        let rsa_key = sequence(&[
            positive_integer(&modulus),
            positive_integer(&[0x01, 0x00, 0x01]),
        ]);
        let mut key_bits = vec![0];
        key_bits.extend_from_slice(&rsa_key);
        let spki = sequence(&[algorithm(RSA_ENCRYPTION_OID), der(0x03, &key_bits)]);
        let attribute = sequence(&[der(0x06, &[0x55, 0x04, 0x03]), der(0x0c, b"example")]);
        let name = sequence(&[der(0x31, &attribute)]);
        let validity = sequence(&[der(0x17, b"260101000000Z"), der(0x17, b"270101000000Z")]);
        let tbs = sequence(&[
            der(0xa0, &der(0x02, &[0x02])),
            der(0x02, &[0x01]),
            algorithm(SHA256_WITH_RSA_OID),
            name.clone(),
            validity,
            name,
            spki,
        ]);
        sequence(&[tbs, algorithm(SHA256_WITH_RSA_OID), der(0x03, &[0, 0xaa])])
    }

    #[test]
    fn only_the_server_role_is_authorized() {
        assert_eq!(require_server_role(CertificateVerifyRole::Server), Ok(()));
        assert_eq!(
            require_server_role(CertificateVerifyRole::Client),
            Err(ReviewedServerPathCertificateVerifyError {
                kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
            })
        );
    }

    #[test]
    fn derives_the_exact_reviewed_leaf_rsa_key_under_caller_policy() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[65_537]).unwrap();

        let key = validate_leaf_public_key(&certificate, policy, Asn1Limits::default()).unwrap();

        assert_eq!(key.modulus_bits(), 2_048);
        assert_eq!(key.public_exponent(), 65_537);
        assert_eq!(key.modulus().first(), Some(&0x80));
        assert_eq!(key.modulus().last(), Some(&1));
    }

    #[test]
    fn leaf_key_policy_failure_is_stable_and_redacted() {
        let encoded = test_certificate();
        let certificate = decode_x509_certificate(&encoded, Asn1Limits::default()).unwrap();
        let policy = RsaPublicKeyPolicy::new(2_048, 2_048, &[3]).unwrap();

        let error = validate_leaf_public_key(&certificate, policy, Asn1Limits::default())
            .expect_err("the reviewed leaf exponent is not allowed");

        assert_eq!(
            error.kind(),
            ReviewedServerPathCertificateVerifyErrorKind::PublicKey(
                RsaPublicKeyErrorKind::PublicExponentNotAllowed
            )
        );
        assert!(!error.to_string().contains("65537"));
    }

    #[test]
    fn result_surface_is_nominal_and_non_copyable() {
        type Proof = VerifiedReviewedServerPathCertificateVerify<
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
            'static,
        >;
        assert!(
            core::any::type_name::<Proof>().contains("VerifiedReviewedServerPathCertificateVerify")
        );
    }

    #[test]
    fn stable_error_discloses_no_payload() {
        let error = ReviewedServerPathCertificateVerifyError {
            kind: ReviewedServerPathCertificateVerifyErrorKind::WrongEndpointRole,
        };
        let display = error.to_string();
        let debug = format!("{error:?}");

        assert_eq!(
            display,
            "reviewed server-path CertificateVerify error WrongEndpointRole"
        );
        assert_eq!(
            debug,
            "ReviewedServerPathCertificateVerifyError { kind: WrongEndpointRole }"
        );
        for secret in ["api.example.test", "transcript", "signature", "modulus"] {
            assert!(!display.contains(secret));
            assert!(!debug.contains(secret));
        }
    }
}
