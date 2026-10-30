//! Exact TLS 1.3 `CertificateVerify` signed-input framing.
//!
//! RFC 8446 section 4.4.3 signs a role-specific context, prefixed by 64 space
//! bytes and terminated by one zero byte, followed by the handshake transcript
//! hash. This crate writes exactly that public input into caller-owned memory.
//! It does not calculate the transcript hash, select a signature scheme, sign,
//! verify, parse certificates, or perform I/O.
//!
//! ```
//! use tls13_certificate_verify_input::{
//!     build_certificate_verify_input, CertificateVerifyRole,
//!     TranscriptHashAlgorithm, MAXIMUM_INPUT_LENGTH,
//! };
//!
//! let transcript_hash = [0x42; 32];
//! let mut output = [0u8; MAXIMUM_INPUT_LENGTH];
//! let input = build_certificate_verify_input(
//!     CertificateVerifyRole::Server,
//!     TranscriptHashAlgorithm::Sha256,
//!     &transcript_hash,
//!     &mut output,
//! )?;
//! assert_eq!(input.as_bytes().len(), 130);
//! # Ok::<(), tls13_certificate_verify_input::InputError>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// Number of space bytes that precede the context string.
pub const SIGNATURE_PAD_LENGTH: usize = 64;

/// The RFC 8446 server `CertificateVerify` context string.
pub const SERVER_CONTEXT: &[u8; 33] = b"TLS 1.3, server CertificateVerify";

/// The RFC 8446 client `CertificateVerify` context string.
pub const CLIENT_CONTEXT: &[u8; 33] = b"TLS 1.3, client CertificateVerify";

/// Exact signed-input length when the transcript hash is SHA-256.
pub const SHA256_INPUT_LENGTH: usize = SIGNATURE_PAD_LENGTH + SERVER_CONTEXT.len() + 1 + 32;

/// Exact signed-input length when the transcript hash is SHA-384.
pub const SHA384_INPUT_LENGTH: usize = SIGNATURE_PAD_LENGTH + SERVER_CONTEXT.len() + 1 + 48;

/// Largest output buffer required by this package.
pub const MAXIMUM_INPUT_LENGTH: usize = SHA384_INPUT_LENGTH;

/// The endpoint whose `CertificateVerify` input is being framed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CertificateVerifyRole {
    /// The server `CertificateVerify` context.
    Server,
    /// The client `CertificateVerify` context.
    Client,
}

impl CertificateVerifyRole {
    /// Return the exact RFC 8446 context string for this role.
    #[must_use]
    pub const fn context(self) -> &'static [u8; 33] {
        match self {
            Self::Server => SERVER_CONTEXT,
            Self::Client => CLIENT_CONTEXT,
        }
    }
}

/// Transcript-hash algorithms supported by the targeted TLS 1.3 suites.
///
/// This is length metadata only. A later composition must bind the value to
/// the negotiated cipher suite and the transcript implementation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TranscriptHashAlgorithm {
    /// SHA-256 with a 32-byte digest.
    Sha256,
    /// SHA-384 with a 48-byte digest.
    Sha384,
}

impl TranscriptHashAlgorithm {
    /// Return the exact digest length accepted for this algorithm.
    #[must_use]
    pub const fn output_length(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
        }
    }

    /// Return the exact signed-input length for this algorithm.
    #[must_use]
    pub const fn input_length(self) -> usize {
        SIGNATURE_PAD_LENGTH + SERVER_CONTEXT.len() + 1 + self.output_length()
    }
}

/// Stable, redacted input-framing failures.
///
/// Errors retain neither caller bytes nor caller lengths.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputError {
    /// The transcript hash did not have the algorithm's exact output length.
    InvalidTranscriptHashLength,
    /// Caller-owned output could not hold the complete signed input.
    OutputTooShort,
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTranscriptHashLength => {
                formatter.write_str("invalid transcript hash length")
            }
            Self::OutputTooShort => formatter.write_str("output buffer is too short"),
        }
    }
}

impl std::error::Error for InputError {}

/// A validated borrowed TLS 1.3 `CertificateVerify` signed input.
///
/// Debug output deliberately omits the transcript hash bytes.
#[derive(Eq, PartialEq)]
pub struct CertificateVerifyInput<'a> {
    role: CertificateVerifyRole,
    transcript_hash_algorithm: TranscriptHashAlgorithm,
    bytes: &'a [u8],
}

impl CertificateVerifyInput<'_> {
    /// Return the endpoint role bound to these bytes.
    #[must_use]
    pub const fn role(&self) -> CertificateVerifyRole {
        self.role
    }

    /// Return the transcript-hash metadata bound to these bytes.
    #[must_use]
    pub const fn transcript_hash_algorithm(&self) -> TranscriptHashAlgorithm {
        self.transcript_hash_algorithm
    }

    /// Borrow the exact bytes to hash and sign.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        self.bytes
    }
}

impl fmt::Debug for CertificateVerifyInput<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CertificateVerifyInput")
            .field("role", &self.role)
            .field("transcript_hash_algorithm", &self.transcript_hash_algorithm)
            .finish_non_exhaustive()
    }
}

/// Frame the exact TLS 1.3 `CertificateVerify` content to hash and sign.
///
/// Both input length and output capacity are validated before `output` is
/// changed. Extra output capacity is left untouched. This function performs no
/// hashing or signing operation.
pub fn build_certificate_verify_input<'a>(
    role: CertificateVerifyRole,
    transcript_hash_algorithm: TranscriptHashAlgorithm,
    transcript_hash: &[u8],
    output: &'a mut [u8],
) -> Result<CertificateVerifyInput<'a>, InputError> {
    if transcript_hash.len() != transcript_hash_algorithm.output_length() {
        return Err(InputError::InvalidTranscriptHashLength);
    }

    let input_length = transcript_hash_algorithm.input_length();
    if output.len() < input_length {
        return Err(InputError::OutputTooShort);
    }

    output[..SIGNATURE_PAD_LENGTH].fill(0x20);

    let context_start = SIGNATURE_PAD_LENGTH;
    let context_end = context_start + role.context().len();
    output[context_start..context_end].copy_from_slice(role.context());
    output[context_end] = 0;
    output[context_end + 1..input_length].copy_from_slice(transcript_hash);

    Ok(CertificateVerifyInput {
        role,
        transcript_hash_algorithm,
        bytes: &output[..input_length],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_sha256_matches_rfc_layout() {
        let transcript_hash = [0x01; 32];
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH];

        let input = build_certificate_verify_input(
            CertificateVerifyRole::Server,
            TranscriptHashAlgorithm::Sha256,
            &transcript_hash,
            &mut output,
        )
        .unwrap();

        assert_eq!(input.role(), CertificateVerifyRole::Server);
        assert_eq!(
            input.transcript_hash_algorithm(),
            TranscriptHashAlgorithm::Sha256
        );
        assert_eq!(input.as_bytes().len(), SHA256_INPUT_LENGTH);
        assert_eq!(&input.as_bytes()[..64], &[0x20; 64]);
        assert_eq!(&input.as_bytes()[64..97], SERVER_CONTEXT);
        assert_eq!(input.as_bytes()[97], 0);
        assert_eq!(&input.as_bytes()[98..], &transcript_hash);
    }

    #[test]
    fn client_sha384_uses_client_context_and_exact_prefix() {
        let transcript_hash = [0x7b; 48];
        let mut output = [0xa5; MAXIMUM_INPUT_LENGTH + 3];

        let input = build_certificate_verify_input(
            CertificateVerifyRole::Client,
            TranscriptHashAlgorithm::Sha384,
            &transcript_hash,
            &mut output,
        )
        .unwrap();

        assert_eq!(input.role(), CertificateVerifyRole::Client);
        assert_eq!(input.as_bytes().len(), SHA384_INPUT_LENGTH);
        assert_eq!(&input.as_bytes()[64..97], CLIENT_CONTEXT);
        assert_eq!(input.as_bytes()[97], 0);
        assert_eq!(&input.as_bytes()[98..], &transcript_hash);
        assert_eq!(&output[MAXIMUM_INPUT_LENGTH..], &[0xa5; 3]);
    }

    #[test]
    fn transcript_hash_lengths_fail_without_mutating_output() {
        for (algorithm, invalid_lengths) in [
            (TranscriptHashAlgorithm::Sha256, [31, 33]),
            (TranscriptHashAlgorithm::Sha384, [47, 49]),
        ] {
            for invalid_length in invalid_lengths {
                let transcript_hash = [0x42; 49];
                let mut output = [0xa5; MAXIMUM_INPUT_LENGTH];

                assert_eq!(
                    build_certificate_verify_input(
                        CertificateVerifyRole::Server,
                        algorithm,
                        &transcript_hash[..invalid_length],
                        &mut output,
                    ),
                    Err(InputError::InvalidTranscriptHashLength)
                );
                assert_eq!(output, [0xa5; MAXIMUM_INPUT_LENGTH]);
            }
        }
    }

    #[test]
    fn short_output_fails_without_mutating_output() {
        for algorithm in [
            TranscriptHashAlgorithm::Sha256,
            TranscriptHashAlgorithm::Sha384,
        ] {
            let transcript_hash = [0x42; 48];
            let mut output = [0xa5; MAXIMUM_INPUT_LENGTH - 1];
            let output_length = algorithm.input_length() - 1;

            assert_eq!(
                build_certificate_verify_input(
                    CertificateVerifyRole::Client,
                    algorithm,
                    &transcript_hash[..algorithm.output_length()],
                    &mut output[..output_length],
                ),
                Err(InputError::OutputTooShort)
            );
            assert_eq!(output, [0xa5; MAXIMUM_INPUT_LENGTH - 1]);
        }
    }

    #[test]
    fn public_lengths_and_contexts_are_exact() {
        assert_eq!(SERVER_CONTEXT.len(), 33);
        assert_eq!(CLIENT_CONTEXT.len(), 33);
        assert_eq!(TranscriptHashAlgorithm::Sha256.output_length(), 32);
        assert_eq!(TranscriptHashAlgorithm::Sha384.output_length(), 48);
        assert_eq!(TranscriptHashAlgorithm::Sha256.input_length(), 130);
        assert_eq!(TranscriptHashAlgorithm::Sha384.input_length(), 146);
        assert_eq!(SHA256_INPUT_LENGTH, 130);
        assert_eq!(SHA384_INPUT_LENGTH, 146);
        assert_eq!(MAXIMUM_INPUT_LENGTH, 146);
    }

    #[test]
    fn debug_output_omits_input_bytes() {
        let transcript_hash = [0x7b; 32];
        let mut output = [0u8; MAXIMUM_INPUT_LENGTH];
        let input = build_certificate_verify_input(
            CertificateVerifyRole::Server,
            TranscriptHashAlgorithm::Sha256,
            &transcript_hash,
            &mut output,
        )
        .unwrap();

        let debug = format!("{input:?}");
        assert!(debug.contains("Server"));
        assert!(debug.contains("Sha256"));
        assert!(!debug.contains("123"));
        assert!(!debug.contains("bytes"));
    }

    #[test]
    fn errors_are_stable_and_redacted() {
        assert_eq!(
            InputError::InvalidTranscriptHashLength.to_string(),
            "invalid transcript hash length"
        );
        assert_eq!(
            InputError::OutputTooShort.to_string(),
            "output buffer is too short"
        );
    }
}
