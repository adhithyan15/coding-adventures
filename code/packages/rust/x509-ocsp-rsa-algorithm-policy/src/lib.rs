//! # Response-bound OCSP RSA algorithm policy
//!
//! Structural decoding tells us which algorithm identifier and signature bits
//! an OCSP response contains. RSA key decoding tells us the public modulus and
//! exponent. Neither boundary decides whether those values belong to one
//! caller-approved deployment profile. This crate supplies that missing pure
//! policy step without selecting or invoking a cryptographic verifier.

#![deny(unsafe_code)]

use core::fmt;
use x509_ocsp_basic_response::X509OcspBasicResponse;
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// Exact parameter-presence policy for one OCSP signature algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaAlgorithmParametersPolicy<'a> {
    Absent,
    ExactDer(&'a [u8]),
}

impl X509OcspRsaAlgorithmParametersPolicy<'_> {
    fn matches(self, actual: Option<der_asn1::Asn1Element<'_>>) -> bool {
        match (self, actual) {
            (Self::Absent, None) => true,
            (Self::ExactDer(expected), Some(actual)) => expected == actual.encoded(),
            (Self::Absent | Self::ExactDer(_), _) => false,
        }
    }
}

/// Stable invalid-profile categories that retain no caller policy values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaAlgorithmProfileDefinitionErrorKind {
    EmptySignatureAlgorithm,
    InvalidModulusBitLengthRange,
    InvalidSignatureBitLengthRange,
    EmptyPublicExponentPolicy,
    InvalidPublicExponentPolicy,
}

/// A redacted caller-profile construction failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaAlgorithmProfileDefinitionError {
    kind: X509OcspRsaAlgorithmProfileDefinitionErrorKind,
}

impl X509OcspRsaAlgorithmProfileDefinitionError {
    const fn new(kind: X509OcspRsaAlgorithmProfileDefinitionErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509OcspRsaAlgorithmProfileDefinitionErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRsaAlgorithmProfileDefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP RSA algorithm profile definition error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRsaAlgorithmProfileDefinitionError {}

/// One exact caller-supplied signature-algorithm and RSA-key profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaAlgorithmProfile<'a> {
    signature_algorithm: &'a [u64],
    signature_parameters: X509OcspRsaAlgorithmParametersPolicy<'a>,
    minimum_modulus_bits: usize,
    maximum_modulus_bits: usize,
    minimum_signature_bits: usize,
    maximum_signature_bits: usize,
    permitted_public_exponents: &'a [u64],
}

impl<'a> X509OcspRsaAlgorithmProfile<'a> {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        signature_algorithm: &'a [u64],
        signature_parameters: X509OcspRsaAlgorithmParametersPolicy<'a>,
        minimum_modulus_bits: usize,
        maximum_modulus_bits: usize,
        minimum_signature_bits: usize,
        maximum_signature_bits: usize,
        permitted_public_exponents: &'a [u64],
    ) -> Result<Self, X509OcspRsaAlgorithmProfileDefinitionError> {
        if signature_algorithm.is_empty() {
            return Err(X509OcspRsaAlgorithmProfileDefinitionError::new(
                X509OcspRsaAlgorithmProfileDefinitionErrorKind::EmptySignatureAlgorithm,
            ));
        }
        if minimum_modulus_bits == 0 || minimum_modulus_bits > maximum_modulus_bits {
            return Err(X509OcspRsaAlgorithmProfileDefinitionError::new(
                X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidModulusBitLengthRange,
            ));
        }
        if minimum_signature_bits == 0 || minimum_signature_bits > maximum_signature_bits {
            return Err(X509OcspRsaAlgorithmProfileDefinitionError::new(
                X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidSignatureBitLengthRange,
            ));
        }
        if permitted_public_exponents.is_empty() {
            return Err(X509OcspRsaAlgorithmProfileDefinitionError::new(
                X509OcspRsaAlgorithmProfileDefinitionErrorKind::EmptyPublicExponentPolicy,
            ));
        }

        let mut index = 0;
        while index < permitted_public_exponents.len() {
            let exponent = permitted_public_exponents[index];
            if exponent < 3
                || exponent.is_multiple_of(2)
                || (index > 0 && permitted_public_exponents[index - 1] >= exponent)
            {
                return Err(X509OcspRsaAlgorithmProfileDefinitionError::new(
                    X509OcspRsaAlgorithmProfileDefinitionErrorKind::InvalidPublicExponentPolicy,
                ));
            }
            index += 1;
        }

        Ok(Self {
            signature_algorithm,
            signature_parameters,
            minimum_modulus_bits,
            maximum_modulus_bits,
            minimum_signature_bits,
            maximum_signature_bits,
            permitted_public_exponents,
        })
    }

    pub const fn signature_algorithm(&self) -> &'a [u64] {
        self.signature_algorithm
    }

    pub const fn signature_parameters(&self) -> X509OcspRsaAlgorithmParametersPolicy<'a> {
        self.signature_parameters
    }

    pub const fn minimum_modulus_bits(&self) -> usize {
        self.minimum_modulus_bits
    }

    pub const fn maximum_modulus_bits(&self) -> usize {
        self.maximum_modulus_bits
    }

    pub const fn minimum_signature_bits(&self) -> usize {
        self.minimum_signature_bits
    }

    pub const fn maximum_signature_bits(&self) -> usize {
        self.maximum_signature_bits
    }

    pub const fn permitted_public_exponents(&self) -> &'a [u64] {
        self.permitted_public_exponents
    }

    fn matches(&self, response: &X509OcspBasicResponse<'_>) -> bool {
        let algorithm = response.signature_algorithm();
        algorithm.algorithm().equals(self.signature_algorithm)
            && self.signature_parameters.matches(algorithm.parameters())
    }
}

/// Stable policy failures that retain no response, key, signature, or profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaAlgorithmPolicyErrorKind {
    UnsupportedAlgorithmProfile,
    AmbiguousAlgorithmProfile,
    ModulusBitLengthOutOfPolicy,
    PublicExponentOutOfPolicy,
    SignatureBitLengthOutOfPolicy,
}

/// A redacted OCSP RSA algorithm-policy failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaAlgorithmPolicyError {
    kind: X509OcspRsaAlgorithmPolicyErrorKind,
}

impl X509OcspRsaAlgorithmPolicyError {
    const fn new(kind: X509OcspRsaAlgorithmPolicyErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> X509OcspRsaAlgorithmPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRsaAlgorithmPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP RSA algorithm policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRsaAlgorithmPolicyError {}

/// Non-copyable policy evidence bound to one exact response and RSA key.
pub struct ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key> {
    response: &'response X509OcspBasicResponse<'encoded>,
    responder_public_key: ValidatedRsaPublicKey<'key>,
    profile_index: usize,
}

impl<'response, 'encoded, 'key> ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key> {
    pub const fn response(&self) -> &'response X509OcspBasicResponse<'encoded> {
        self.response
    }

    pub const fn responder_public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.responder_public_key
    }

    pub const fn profile_index(&self) -> usize {
        self.profile_index
    }
}

impl fmt::Debug for ValidatedX509OcspRsaAlgorithmPolicy<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedX509OcspRsaAlgorithmPolicy")
            .field("profile_index", &self.profile_index)
            .finish_non_exhaustive()
    }
}

/// Select exactly one caller-supplied profile and bind it to the exact inputs.
pub fn validate_x509_ocsp_rsa_algorithm_policy<'response, 'encoded, 'key, 'policy>(
    response: &'response X509OcspBasicResponse<'encoded>,
    responder_public_key: ValidatedRsaPublicKey<'key>,
    profiles: &[X509OcspRsaAlgorithmProfile<'policy>],
) -> Result<
    ValidatedX509OcspRsaAlgorithmPolicy<'response, 'encoded, 'key>,
    X509OcspRsaAlgorithmPolicyError,
> {
    let mut selected = None;
    for (profile_index, profile) in profiles.iter().enumerate() {
        if profile.matches(response) {
            if selected.is_some() {
                return Err(X509OcspRsaAlgorithmPolicyError::new(
                    X509OcspRsaAlgorithmPolicyErrorKind::AmbiguousAlgorithmProfile,
                ));
            }
            selected = Some((profile_index, profile));
        }
    }

    let Some((profile_index, profile)) = selected else {
        return Err(X509OcspRsaAlgorithmPolicyError::new(
            X509OcspRsaAlgorithmPolicyErrorKind::UnsupportedAlgorithmProfile,
        ));
    };

    if !(profile.minimum_modulus_bits..=profile.maximum_modulus_bits)
        .contains(&responder_public_key.modulus_bits())
    {
        return Err(X509OcspRsaAlgorithmPolicyError::new(
            X509OcspRsaAlgorithmPolicyErrorKind::ModulusBitLengthOutOfPolicy,
        ));
    }
    if !profile
        .permitted_public_exponents
        .contains(&responder_public_key.public_exponent())
    {
        return Err(X509OcspRsaAlgorithmPolicyError::new(
            X509OcspRsaAlgorithmPolicyErrorKind::PublicExponentOutOfPolicy,
        ));
    }
    if !(profile.minimum_signature_bits..=profile.maximum_signature_bits)
        .contains(&response.signature().bit_len())
    {
        return Err(X509OcspRsaAlgorithmPolicyError::new(
            X509OcspRsaAlgorithmPolicyErrorKind::SignatureBitLengthOutOfPolicy,
        ));
    }

    Ok(ValidatedX509OcspRsaAlgorithmPolicy {
        response,
        responder_public_key,
        profile_index,
    })
}
